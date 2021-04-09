use std::convert::Infallible;
use std::sync::{Mutex, MutexGuard};

use async_trait::async_trait;

use cucumber::Steps;
use lazy_static::lazy_static;
use uom::si::{f64::*, length::foot, velocity::foot_per_minute};
use xng_rs::prelude::*;

mod util;

use super::*;
use util::*;

const NUM_TESTS: usize = 100;

lazy_static! {
    static ref TAWS_IF: Mutex<(
        port::SamplingReceiver::<ALERT_STATE_SIZE>,
        port::SamplingSender::<AIRCRAFT_STATE_SIZE>
    )> = Mutex::new((
        port::SamplingReceiver::<ALERT_STATE_SIZE>::new(
            cstr!("taws::alerts"),
            Duration::from_secs(1)
        )
        .unwrap(),
        port::SamplingSender::<AIRCRAFT_STATE_SIZE>::new(cstr!("aircraft_state")).unwrap()
    ));
}

struct MyWorld {
    lock: MutexGuard<
        'static,
        (
            port::SamplingReceiver<ALERT_STATE_SIZE>,
            port::SamplingSender<AIRCRAFT_STATE_SIZE>,
        ),
    >,
    moulds: Vec<Box<dyn FnMut(&mut AircraftState)>>,
}

impl MyWorld {
    pub fn add_mould<F: 'static + FnMut(&mut AircraftState)>(&mut self, f: F) {
        self.moulds.push(Box::new(f));
    }
}

#[async_trait(?Send)]
impl cucumber::World for MyWorld {
    type Error = Infallible;

    async fn new() -> Result<Self, Infallible> {
        Ok(Self {
            lock: TAWS_IF.lock().unwrap(),
            moulds: Vec::new(),
        })
    }
}

pub fn test() {
    let runner = cucumber::Cucumber::<MyWorld>::new()
        .features(&["features"])
        .steps(steps());

    futures::executor::block_on(runner.run());
}

fn steps() -> Steps<MyWorld> {
    let mut builder: Steps<MyWorld> = Steps::new();
    builder
        .given("the plane is flying", |world, _step| world)
        .given_regex(r#"^(.+) is (.*)armed$"#, |world, mut matches, _step| {
            matches[1].retain(|c| !c.is_whitespace());
            let alert_system = parse_alert(&matches[1]);
            let mut taws_input = TawsInput::default();
            taws_input.arm[0]= Some((alert_system, !matches[2].starts_with("not")));
            taws_input.send(&world.lock.1).unwrap();
            world
        })
        .given_regex(
            "^(.+) is (.*)inhibited$",
            |world, mut matches, _step| {
                matches[1].retain(|c| !c.is_whitespace());
                let alert_system = parse_alert(&matches[1]);
                let mut taws_input = TawsInput::default();
                taws_input.inhibit[0]= Some((alert_system, !matches[2].starts_with("not")));
                taws_input.send(&world.lock.1).unwrap();
                world
            },
        )
        .given_regex(
            r"^steep approach is (.*)selected$",
            |mut world, matches, _step| {
                if matches[1].starts_with("not") {
                    world.add_mould(|a| a.steep_approach = false);
                } else {
                    world.add_mould(|a| a.steep_approach = true);
                }
                world
            },
        )
        .when_regex(
            r"^the rate of descent is at (\w+) (\d+) feet per minute$",
            |mut world, matches, _step| {
                let rod = Velocity::new::<foot_per_minute>(matches[2].parse().unwrap());
                let mut bouncer = BouncingClamp();
                // most and least are swapped here, as aircraft_state stores rate of climb, while
                // the sentence give rate of descent 
                // TODO validate that this is a safe assumption?
                match matches[1].as_str() {
                    "most" => {
                        world.add_mould( move |a| bouncer.at_least(&mut a.climb_rate, -rod));
                    }
                    "least" => {
                        world.add_mould( move |a| bouncer.at_most(&mut a.climb_rate, -rod));
                    }
                    _ => {
                        panic!("unable to parse this sentence");
                    }
                }
                world
            },
        )
        .when_regex(
            r"^the height above terrain is (.*)between (\d+) and (\d+) feet$",
            |mut world, matches, _step| {
                let height_at_least = Length::new::<foot>(matches[2].parse().unwrap());
                let height_at_most = Length::new::<foot>(matches[3].parse().unwrap());

                let mut bouncer = BouncingClamp();

                if matches[1].starts_with("not") {
                    world.add_mould(move |a| bouncer.not_in_range(
                        &mut a.altitude_ground,
                        height_at_least,
                        height_at_most
                    ));
                } else {
                    world.add_mould( move |a| bouncer.in_range(
                        &mut a.altitude_ground,
                        height_at_least,
                        height_at_most
                    )); // TODO altitude or altitude_ground
                }
                world
            },
        )
        .then_regex(
            "^a (.*) alert is not emitted at all$",
            |mut world, matches, _step| {
                let (alert,level) = parse_alert_level(&matches[1]);

                let mut aircraft_states: Vec<_> = AircraftStateGenerator::default()
                    .take(NUM_TESTS)
                    .collect();

                // press the test data in our moulds
                for frame in aircraft_states.iter_mut() {
                    for f in world.moulds.iter_mut() {
                        f(frame);
                    }
                }

                let mut taws_input = TawsInput::default();
                for frame in aircraft_states {
                    // send data to other partition
                    taws_input.aircraft_state = frame.clone();
                    taws_input.send(&world.lock.1).unwrap();
                    xng_rs::vcpu::finish_slot();
                    let mut buf = [0u8; ALERT_STATE_SIZE];
                    let (recieved_buf, _) = world.lock.0.recv(&mut buf).unwrap().unwrap();
                    let alert_state: AlertState = postcard::from_bytes(recieved_buf).unwrap();

                    if alert_state.iter().any(|(a, l)| a == alert && l <= level)
                    {
                        panic!("Aicraft state that violated the scenario: {:#?}\nalerts emitted: {:#?}", frame, alert_state);
                    }
                }
                world
            },
        )
        .then_regex(
            r"^a (.*) alert is emitted within (\d+) seconds$",
            |mut world, matches, _step| {
                let (alert,level) = parse_alert_level(&matches[1]);

                let mut aircraft_states: Vec<_> = AircraftStateGenerator::default()
                    .take(NUM_TESTS)
                    .collect();

                // press the test data in our moulds
                for frame in aircraft_states.iter_mut() {
                    for f in world.moulds.iter_mut() {
                        f(frame);
                    }
                }

                let mut taws_input = TawsInput::default();

                for frame in aircraft_states {
                    // send data to other partition
                    taws_input.aircraft_state = frame.clone();
                    taws_input.send(&world.lock.1).unwrap();
                    xng_rs::vcpu::finish_slot();
                    let mut buf = [0u8; ALERT_STATE_SIZE];
                    let (recieved_buf, _) = world.lock.0.recv(&mut buf).unwrap().unwrap();
                    let alert_state: AlertState = postcard::from_bytes(recieved_buf).unwrap();

                    // TODO what about the time constraint?
                    // Count all alerts that are from the functionality Mode1 and are of higher or
                    // same priority as `level`. If the count is 0, the system did not alert
                    // appropiately.
                    if alert_state
                        .iter()
                        .filter(|(a, l)| *a  == alert && *l <= level)
                        .count()
                        == 0
                    {
                        panic!("Aicraft state that violated the scenario: {:#?}\nalerts emitted: {:#?}", frame, alert_state);
                    }
                }
                world
            },
        );
    builder
}
