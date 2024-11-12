use std::collections::HashMap;
use std::io::empty;
use std::iter::Map;
use std::usize;
use crate::Opts;
use anyhow::Result;
use sysfs_pwm::{Pwm, PwmChip};
use crate::led_pwm::PwmLedSection::{Chassis, Ring, Side};

const PWM_PERIOD: u32 = 50_000;
const PWM_DEFAULT_DUTY_CYCLE: u32 = PWM_PERIOD;
const PWM_SPEED_MAPPING:[u32;256] = { // Generate our mappings at compile time; since it has a full capability of 256 different strengths.
    const T: u32 = 0;
    let mut out = [T;256];
    let mut i = 0;

    loop {
        if i >= 256 {
            break;
        }

        out[i] = (PWM_PERIOD as f32 * ((i as f32) / 256f32)) as u32;
        i += 1;
    }

    out
};

#[derive(Hash, Eq, PartialEq)]
enum PwmLedSection {
    Ring,
    Chassis,
    Side,
}

struct PwmLedConfig {
    map: HashMap<PwmLedSection, Vec<Pwm>>,
}

fn create_config(
    ring: Option<Vec<String>>,
    side: Option<Vec<String>>,
    chassis: Option<Vec<String>>,
) -> Result<PwmLedConfig> {
    let mut out = PwmLedConfig{
        map: HashMap::new(),
    };

    fn apply_item(pin: Option<Vec<String>>, section: PwmLedSection, map: &mut HashMap<PwmLedSection, Vec<Pwm>>,) {
        let out: Vec<Pwm> = Vec::new();
        match pin {
            Some(pins) => {
                pins.iter().for_each(|pinstr| {
                    let parts = pinstr.split('-');
                })
                // parse

                // insert into list
                // insert list into map

            }
            None => {
                // No-op
            }
        }
    }

    apply_item(ring, Ring, &mut out.map);
    apply_item(side, Chassis, &mut out.map);
    apply_item(chassis, Side, &mut out.map);

    match initialize_pins(&out) {
        Ok(()) => {}
        Err(e) => return Err(e),
    }

    Ok(out)
}

fn initialize_pins(cfg: &PwmLedConfig) -> Result<()> {
    // for key in cfg.map.keys() {
    //     // Export it in pwm
    //     // Set period
    //     // Enable
    // }
    //
    // // Set the pins all up at max default
    // update_pins([255,255,255], cfg);

    for (_, pins) in cfg.map.iter() {
        for pin in pins {
            pin.export()?; // Export if it isn't already
            pin.set_period_ns(PWM_PERIOD)?; // Configure the period
            pin.set_duty_cycle_ns(PWM_PERIOD)?; // Max the duty cycle, we want to start live.
            pin.enable(true)?; // Go!
        }
    }

    Ok(())
}

fn close_pins(cfg: &PwmLedConfig) -> Result<()> {
    for (_, pins) in cfg.map.iter() {
        for pin in pins {
            pin.set_duty_cycle_ns(0)?; // Fully off so that next time it comes up, it must be configured.
            pin.enable(false)?; // Disable the pin, reducing output to true zero.
            pin.unexport()?; // Unexport.
        }
    }

    Ok(())
}

fn update_pins(
    fet_packet: [u8; 3], // FET0: Chassis; FET1: Ring; FET2: Side
    cfg: &PwmLedConfig,
) -> Result<()> {
    struct avgData {
        count: u8,
        sum: u32,
    }

    // Create our averages
    let mut outputs: HashMap<Pwm, avgData> = HashMap::new();

    fn apply_packet(section: PwmLedSection, value: u8, cfg: &PwmLedConfig) {
        match cfg.map.get(&section) {
            Some(pins) => {
                pins.iter().for_each(|x| {
                    outputs[x].sum += value as u32;
                    outputs[x].count += 1;
                })
            }
            None => {} // no-op
        }
    }

    apply_packet(Chassis, fet_packet[0], cfg);
    apply_packet(Ring, fet_packet[1], cfg);
    apply_packet(Side, fet_packet[2], cfg);

    outputs.iter().for_each(|pwm, avg| {
        let duty_cycle = PWM_SPEED_MAPPING[(avg.sum / avg.count) as u8]; // Average will always be [0,255], so this conversion is safe
        pwm.set_duty_cycle_ns(duty_cycle); // Change LED juicing
    });

    Ok(())
}