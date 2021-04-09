use std::collections::HashMap;
use std::io::{stdout, Write};

use xng_rs::prelude::*;

use opentaws::prelude::*;
use ordered_float::NotNan;
use p_taws::*;
use rtlola_frontend::FrontendConfig;
use rtlola_interpreter::{Config, EvalConfig, TimeFormat, TimeRepresentation, Value};

const SPEC: &'static str = include_str!("../../rtlola/spec.lola");

#[allow(non_snake_case)]
#[no_mangle]
pub extern "C" fn PartitionMain() -> isize {
    let frontend_config = FrontendConfig::default();
    let ir = rtlola_frontend::parse("rtlola/spec.lola", SPEC, frontend_config).unwrap();
    let trigger_map: HashMap<_, _> = ir
        .triggers
        .iter()
        .map(|tr| (tr.reference.out_ix(), tr.message.clone()))
        .collect();

    let eval_config = EvalConfig::api(TimeRepresentation::Relative(TimeFormat::FloatSecs));
    let mut monitor = Config::new_api(eval_config, ir).into_monitor().unwrap();

    let aircraft_state_port = port::SamplingReceiver::<AIRCRAFT_STATE_SIZE>::new(
        cstr!("aircraft_state"),
        Duration::from_secs(10),
    )
    .unwrap();
    let taws_alert_port = port::SamplingReceiver::<ALERT_STATE_SIZE>::new(
        cstr!("taws::alerts"),
        Duration::from_secs(10),
    )
    .unwrap();

    let mut buf = [0u8; BUF_SIZE];

    let mut last_input = None;
    let mut last_output = None;

    loop {
        let time_stamp = time::since_boot().unwrap();
        let mut events = vec![Value::None; 2];

        if let Some((buf, _)) = aircraft_state_port.recv(&mut buf).unwrap() {
            let _taws_input: TawsInput = postcard::from_bytes(&buf).unwrap(); // TOOD handle error
            let maybe_ts = aircraft_state_port.status().unwrap().last_message_ts;

            if maybe_ts != last_input {
                if let Some(ts) = maybe_ts {
                    events[0] = Value::Float(NotNan::new(ts.as_secs_f64()).unwrap());
                    last_input = Some(ts);
                }
            }
        }

        if let Some((buf, _)) = taws_alert_port.recv(&mut buf).unwrap() {
            let _taws_alerts: AlertState = postcard::from_bytes(&buf).unwrap(); // TOOD handle error
            let maybe_ts = taws_alert_port.status().unwrap().last_message_ts;

            if maybe_ts != last_output {
                if let Some(ts) = maybe_ts {
                    events[0] = Value::Float(NotNan::new(ts.as_secs_f64()).unwrap());
                    last_output = Some(ts);
                }
            }
        }

        let update = monitor.accept_event(events, time_stamp);
        for trigger_msg in update
            .event
            .iter()
            .filter(|(_, v)| *v == Value::Bool(true))
            .map(|(k, _)| trigger_map.get(k).unwrap())
        {
            println!("\nRTLola trigger: {}\n", trigger_msg);
            stdout().flush().unwrap();
        }

        // TODO this is ugly, but necessary?
        xng_rs::vcpu::finish_slot();
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse_provided_spec() {
        let frontend_config = FrontendConfig::default();
        let ir = rtlola_frontend::parse("rtlola/spec.lola", SPEC, frontend_config);
        if let Err(e) = ir {
            panic!("{}", e);
        }
    }
}
