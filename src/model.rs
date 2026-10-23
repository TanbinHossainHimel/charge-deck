//! Plain data types shared by all providers and the window.
//! No device code lives here, so it is easy to unit test.

/// What kind of gadget it is (decides the icon on the card).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeviceKind {
    Keyboard,
    Mouse,
    Headset,
    Controller,
    Other,
}

/// One successful battery measurement.
#[derive(Clone, Debug, PartialEq)]
pub struct Reading {
    /// 0..=100
    pub percent: u8,
    /// true when we only know roughly (voltage curve, or a few "bars").
    /// The card shows a "≈" in front of the number.
    pub estimated: bool,
    /// Battery voltage in millivolts, if the device told us.
    pub voltage_mv: Option<u32>,
    pub charging: bool,
}

impl Reading {
    pub fn exact(percent: u8, charging: bool) -> Self {
        Reading { percent: percent.min(100), estimated: false, voltage_mv: None, charging }
    }
    pub fn estimated(percent: u8, charging: bool) -> Self {
        Reading { percent: percent.min(100), estimated: true, voltage_mv: None, charging }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Status {
    /// We got a number.
    Ok(Reading),
    /// The receiver answered but the device is asleep / switched off.
    Asleep,
    /// Not measured yet.
    Checking,
}

/// What a provider returns for each device it finds.
#[derive(Clone, Debug, PartialEq)]
pub struct DeviceBattery {
    /// Stable id so a card keeps its place between refreshes.
    pub id: String,
    pub name: String,
    pub kind: DeviceKind,
    /// Shown on the card: "Bluetooth", "ASUS receiver", ...
    pub source: &'static str,
    pub status: Status,
}

/// One card in the window = a device plus "is it still here?" bookkeeping.
#[derive(Clone, Debug)]
pub struct Card {
    pub device: DeviceBattery,
    /// Which provider owns this card (so only that provider can "lose" it).
    pub provider: &'static str,
    pub present: bool,
    /// "HH:MM" of the last scan that saw the device.
    pub last_seen: String,
}

/// All cards of the session.
#[derive(Default, Debug)]
pub struct Deck {
    pub cards: Vec<Card>,
}

impl Deck {
    /// Merge the results of one provider scan.
    /// `now` is "HH:MM" (passed in so tests don't need a clock).
    /// Devices of this provider that are missing from `found` stay on the
    /// deck, greyed out, remembering when they were last seen.
    pub fn apply(&mut self, provider: &'static str, found: Vec<DeviceBattery>, now: &str) {
        for card in self.cards.iter_mut().filter(|c| c.provider == provider) {
            card.present = false;
        }
        for dev in found {
            if let Some(card) = self.cards.iter_mut().find(|c| c.device.id == dev.id) {
                card.device = dev;
                card.provider = provider;
                card.present = true;
                card.last_seen = now.to_string();
            } else {
                self.cards.push(Card { device: dev, provider, present: true, last_seen: now.to_string() });
            }
        }
    }

    /// Cards in display order: lowest battery first, unknown levels next,
    /// missing devices last. Ties keep a stable order by name.
    pub fn sorted(&self) -> Vec<&Card> {
        let mut v: Vec<&Card> = self.cards.iter().collect();
        v.sort_by_key(|c| {
            let level = match (&c.device.status, c.present) {
                (_, false) => (2u8, 0u8),
                (Status::Ok(r), true) => (0, r.percent),
                (_, true) => (1, 0),
            };
            (level, c.device.name.clone())
        });
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dev(id: &str, pct: Option<u8>) -> DeviceBattery {
        DeviceBattery {
            id: id.into(),
            name: id.into(),
            kind: DeviceKind::Other,
            source: "Test",
            status: match pct {
                Some(p) => Status::Ok(Reading::exact(p, false)),
                None => Status::Asleep,
            },
        }
    }

    #[test]
    fn sorts_lowest_first_then_unknown() {
        let mut d = Deck::default();
        d.apply("T", vec![dev("a", Some(80)), dev("b", None), dev("c", Some(10))], "10:00");
        let names: Vec<_> = d.sorted().iter().map(|c| c.device.id.clone()).collect();
        assert_eq!(names, ["c", "a", "b"]);
    }

    #[test]
    fn missing_device_stays_greyed_with_last_seen() {
        let mut d = Deck::default();
        d.apply("T", vec![dev("a", Some(50)), dev("b", Some(60))], "10:00");
        d.apply("T", vec![dev("a", Some(49))], "10:02");
        let b = d.cards.iter().find(|c| c.device.id == "b").unwrap();
        assert!(!b.present);
        assert_eq!(b.last_seen, "10:00");
        // missing cards go to the bottom
        assert_eq!(d.sorted().last().unwrap().device.id, "b");
    }

    #[test]
    fn other_providers_cards_are_untouched() {
        let mut d = Deck::default();
        d.apply("T1", vec![dev("a", Some(50))], "10:00");
        d.apply("T2", vec![], "10:01");
        assert!(d.cards[0].present);
    }
}
