//! charge-deck-probe: console debug tool.
//!
//! Usage:
//!   charge-deck-probe            list HID interfaces from known peripheral brands
//!   charge-deck-probe --all      list every HID interface
//!   charge-deck-probe --verbose  also run every provider once, printing all bytes
//!
//! SAFETY (rules 4 and 6): listing only reads the Windows device list; it never
//! opens or sends anything. Providers run only in --verbose mode, and each one
//! only talks to devices it explicitly knows.

use charge_deck::hid_util::set_verbose;
use charge_deck::model::Status;
use charge_deck::providers::{registry, run_guarded};
use hidapi::HidApi;

/// Vendors of common wireless peripherals (used to hide unrelated HID noise).
const KNOWN_VENDORS: &[(u16, &str)] = &[
    (0x0B05, "ASUS"),
    (0x1532, "Razer"),
    (0x046D, "Logitech"),
    (0x1038, "SteelSeries"),
    (0x1B1C, "Corsair"),
    (0x03F0, "HP / HyperX"),
    (0x0951, "Kingston / HyperX"),
    (0x258A, "Glorious"),
    (0x3554, "Pulsar / generic"),
    (0x093A, "Pixart (OEM mice)"),
    (0x045E, "Microsoft"),
    (0x054C, "Sony"),
    (0x057E, "Nintendo"),
];

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let all = args.iter().any(|a| a == "--all");
    let verbose = args.iter().any(|a| a == "--verbose");

    println!("Charge Deck probe v{}", env!("CARGO_PKG_VERSION"));
    println!("Paste this whole output into a GitHub issue.\n");

    let mut api = match HidApi::new() {
        Ok(a) => a,
        Err(e) => {
            println!("Could not start HID: {e}");
            return;
        }
    };
    let _ = api.refresh_devices();
    let providers = registry();

    println!("== HID interfaces ({}) ==", if all { "all" } else { "known brands; use --all for everything" });
    for d in api.device_list() {
        let vendor = KNOWN_VENDORS.iter().find(|(v, _)| *v == d.vendor_id()).map(|(_, n)| *n);
        if !all && vendor.is_none() {
            continue;
        }
        let claimed = providers
            .iter()
            .find(|p| p.claims(d))
            .map(|p| p.name())
            .unwrap_or("-");
        println!(
            "{:04X}:{:04X} if={:<2} usage_page={:#06x} usage={:#06x} | {} | {} | claimed by: {}",
            d.vendor_id(),
            d.product_id(),
            d.interface_number(),
            d.usage_page(),
            d.usage(),
            d.manufacturer_string().unwrap_or("?"),
            d.product_string().unwrap_or("?"),
            claimed
        );
    }

    if verbose {
        set_verbose(true);
        println!("\n== Running providers (> sent, < received) ==");
        for mut p in providers {
            println!("[{}]", p.name());
            for dev in run_guarded(p.as_mut(), &api) {
                println!("  {} ({:?}) -> {}", dev.name, dev.kind, describe(&dev.status));
            }
        }
    } else {
        println!("\nRun with --verbose to query batteries and show every byte.");
    }
}

fn describe(s: &Status) -> String {
    match s {
        Status::Ok(r) => format!(
            "{}{}%{}{}",
            if r.estimated { "~" } else { "" },
            r.percent,
            r.voltage_mv.map(|v| format!(" {:.2} V", v as f32 / 1000.0)).unwrap_or_default(),
            if r.charging { " (charging)" } else { "" }
        ),
        Status::Asleep => "asleep / off".into(),
        Status::Checking => "checking".into(),
    }
}
