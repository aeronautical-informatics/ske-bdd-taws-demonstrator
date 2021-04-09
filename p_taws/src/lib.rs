use opentaws::prelude::*;
use xng_rs::prelude::*;

use serde::{Deserialize, Serialize};

pub const N_ALERTSYSTEMS: usize = 8;

pub const AIRCRAFT_STATE_SIZE: usize = 128;
pub const ALERT_STATE_SIZE: usize = 16;
pub const BUF_SIZE: usize = AIRCRAFT_STATE_SIZE;

#[derive(Default, Serialize, Deserialize)]
pub struct TawsInput {
    pub arm: [Option<(Alert, bool)>; N_ALERTSYSTEMS],
    pub inhibit: [Option<(Alert, bool)>; N_ALERTSYSTEMS],
    pub aircraft_state: AircraftState,
}

impl TawsInput {
    pub fn send<const N: usize>(&self, tx: &port::SamplingSender<N>) -> Result<(), XngError> {
        let mut buf = [0u8; N];
        let written_buf =
            postcard::to_slice(&self, &mut buf).map_err(|_| XngError::InvalidParam)?;
        tx.send(written_buf)?;
        xng_rs::vcpu::finish_slot();
        Ok(())
    }

    /*
    fn recv<const N: usize>(&self, rx: &port::SamplingReceiver<N>)->Result<Self, XngError>{
        let mut buf = [0u8; N];
        let read_bytes = rx.recv(&mut buf).ok_or(XngError::NotAvailable);
        postcard::from_bytes(&read_bytes).map_err(|_| XngError::InvalidParam)
    }
    */
}

/// This function wraps or entry and yield back if an error occures
#[allow(non_snake_case)]
#[no_mangle]
pub extern "C" fn PartitionMain() -> isize {
    if let Err(e) = entry() {
        println!("{:?}", e);
    }

    xng_rs::partition::halt(partition::my_id().unwrap()).unwrap();
    0
}

pub fn entry() -> Result<(), XngError> {
    //xng_rs::console::write(cstr!("Hallo"))?;
    let my_id = partition::my_id().unwrap();
    println!("Launching taws on partition {}", my_id);

    let taws_config = TawsConfig::default();
    let mut taws = Taws::new(taws_config);

    let aircraft_state_port = port::SamplingReceiver::<AIRCRAFT_STATE_SIZE>::new(
        cstr!("aircraft_state"),
        Duration::from_secs(10),
    )
    .unwrap();
    let taws_alert_port =
        port::SamplingSender::<ALERT_STATE_SIZE>::new(cstr!("taws::alerts")).unwrap();

    println!("TAWS ready");
    let mut buf = [0u8; BUF_SIZE];
    loop {
        // check if we can process an aircraft state
        if let Some((buf, _)) = aircraft_state_port.recv(&mut buf)? {
            let taws_input: TawsInput = postcard::from_bytes(&buf).unwrap(); // TOOD handle error

            for action in &taws_input.arm {
                match action {
                    Some((alert, arm)) if *arm => taws.arm(*alert),
                    Some((alert, arm)) if !*arm => taws.disarm(*alert),
                    _ => {}
                }
            }

            for action in &taws_input.inhibit {
                match action {
                    Some((alert, inhibit)) if *inhibit => taws.inhibit(*alert),
                    Some((alert, inhibit)) if !*inhibit => taws.uninhibit(*alert),
                    _ => {}
                }
            }

            let alerts = taws.process(&taws_input.aircraft_state);
            let buf_to_send = postcard::to_slice(&alerts, buf)
                .map_err(|_| XngError::InvalidParam)
                .unwrap();
            taws_alert_port.send(buf_to_send).unwrap();
        }

        // yield back to the scheduler
        xng_rs::vcpu::finish_slot();
    }
}

#[cfg(test)]
mod test {
    use super::*;

    const BIG_NUMBER: usize = 1000;

    #[test]
    fn check_aircraft_state_size() {
        let mut buf = vec![0u8; BIG_NUMBER];
        let written_bytes = postcard::to_slice(&TawsInput::default(), &mut buf).unwrap();
        if written_bytes.len() > AIRCRAFT_STATE_SIZE {
            panic!(
                "AIRCRAFT_STATE_SIZE needs to be at least {}",
                written_bytes.len()
            );
        }
    }

    #[test]
    fn check_alert_state_size() {
        let mut buf = vec![0u8; BIG_NUMBER];
        let written_bytes = postcard::to_slice(&AlertState::default(), &mut buf).unwrap();
        if written_bytes.len() > ALERT_STATE_SIZE {
            panic!(
                "ALERT_STATE_SIZE needs to be at least {}",
                written_bytes.len()
            );
        }
    }
}
