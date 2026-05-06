#[macro_use]
extern crate log;
#[macro_use]
extern crate clap;
#[macro_use]
extern crate iced;

use chrono;
use clap::Parser;
use iced::widget::{column, container, row, text};
use iced::{time, Element, Subscription};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Parser, Debug)]
#[command(
    name=crate_name!(),
    author=crate_authors!(),
    version=crate_version!(),
    about=crate_description!(),
    long_about=None)
]
struct Args {
    /// Turn on Assmodan timing
    #[arg(short, long, default_value_t = false)]
    asmodan: bool,
    /// Turn on Realm Walker timing
    #[arg(short, long, default_value_t = false)]
    realm_walker: bool,
    /// Turn on debug output
    #[arg(short = 'D', long)]
    debug: bool,
}

static LOGGER: GlobalLogger = GlobalLogger;
// World Boss
const WB_INIT: u64 = 1708381800;
const WB_EVERY: u64 = 60 * 210; // Every 3.5 hours
                                // Legion Event
const LE_INIT: u64 = 1708381200;
const LE_EVERY: u64 = 60 * 25; // Every 25 minutes
                               // Realm Walker
const RW_INIT: u64 = 1728414300;
const RW_EVERY: u64 = 60 * 15; // Every 15 minutes
                               // Assmodan
const AS_INIT: u64 = 1768855500;
const AS_EVERY: u64 = 60 * 210; // Every 3.5 hours
                                // Helltide: starts at top of every hour, lasts 55 minutes
const HT_DURATION: u64 = 60 * 55;

enum EventType {
    WB,
    LE,
    RW,
    ASS,
}

#[derive(Debug, Clone, Copy)]
enum Message {
    Tick,
}

#[derive(Debug, Default)]
struct Counts {
    wb: u64,
    le: u64,
    rw: u64,
    ass: u64,
    ht_active: bool,
    ht_secs: u64,
    asmodan: bool,
    realm_walker: bool,
}

impl Counts {
    fn new(asmodan: bool, realm_walker: bool) -> Self {
        return Self {
            wb: 0,
            le: 0,
            rw: 0,
            ass: 0,
            ht_active: false,
            ht_secs: 0,
            asmodan,
            realm_walker,
        };
    }
}

struct GlobalLogger;

impl log::Log for GlobalLogger {
    fn enabled(&self, meta: &log::Metadata) -> bool {
        return meta.level() <= log::max_level();
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            let d = chrono::Local::now();
            eprintln!(
                "{} - {} - {}:{} {} - {}",
                d.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
                record.level(),
                record.file().unwrap(),
                record.line().unwrap(),
                record.target(),
                record.args(),
            );
        }
    }

    fn flush(&self) {}
}

fn get_args() -> Args {
    return Args::parse();
}

fn setup_logging(args: &Args) {
    let l = if args.debug {
        log::LevelFilter::Debug
    } else {
        log::LevelFilter::Info
    };

    log::set_logger(&LOGGER).unwrap();
    log::set_max_level(l);
}

/// Returns seconds until the next occurrence of the given periodic event.
fn calc_delta(ev: EventType, ts: Option<u64>) -> u64 {
    let now = match ts {
        Some(t) => t,
        None => SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    };

    let (elapsed, every) = match ev {
        EventType::WB => ((now - WB_INIT) % WB_EVERY, WB_EVERY),
        EventType::LE => ((now - LE_INIT) % LE_EVERY, LE_EVERY),
        EventType::RW => ((now - RW_INIT) % RW_EVERY, RW_EVERY),
        EventType::ASS => ((now - AS_INIT) % AS_EVERY, AS_EVERY),
    };

    let delta = every - elapsed;
    debug!("Got delta of: {delta}");
    return delta;
}

/// Returns (active, secs) for the current helltide state.
/// When active, secs is the time remaining in the helltide.
/// When inactive, secs is the time until the next helltide starts.
fn calc_helltide(ts: Option<u64>) -> (bool, u64) {
    let now = match ts {
        Some(t) => t,
        None => SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    };
    let secs_in_hour = now % 3600;

    if secs_in_hour < HT_DURATION {
        (true, HT_DURATION - secs_in_hour)
    } else {
        (false, 3600 - secs_in_hour)
    }
}

/// Formats H:MM:SS for events that may span multiple hours.
fn get_hms(delta: u64) -> String {
    let mut hours: u64 = 0;
    let seconds = delta % 60;
    let mut mins = (delta - seconds) / 60;

    if mins >= 60 {
        hours = mins / 60;
        mins %= 60;
    }

    return format!("  {hours}:{mins:02}:{seconds:02}  ");
}

/// Formats MM:SS for helltide display.
/// During an active helltide the value is prefixed with '-'.
/// During the gap it is shown as a plain positive countdown.
fn get_helltide_str(active: bool, secs: u64) -> String {
    let mins = secs / 60;
    let s = secs % 60;
    if active {
        format!("    -{mins:02}:{s:02}  ")
    } else {
        format!("  0:{mins:02}:{s:02}  ")
    }
}

fn get_color(delta: u64) -> iced::Color {
    if delta <= 300 {
        return color!(0xff8080); // Red
    } else if delta <= 600 {
        return color!(0xffff00); // Yellow
    } else {
        return color!(0x808080); // Gray
    }
}

fn get_helltide_color(active: bool, secs: u64) -> iced::Color {
    debug!("Helltide active: {active}, secs: {secs}");
    if active && secs > 600 {
        color!(0xff6600) // Orange — helltide is live
    } else if active && secs <= 300 {
        color!(0xff8080) // Red
    } else if active && secs <= 600 {
        color!(0xffff00) // Yellow
    } else {
        color!(0x808080) // Gray
    }
}

fn update(counts: &mut Counts, _: Message) {
    counts.wb = calc_delta(EventType::WB, None);
    counts.le = calc_delta(EventType::LE, None);
    counts.rw = calc_delta(EventType::RW, None);
    counts.ass = calc_delta(EventType::ASS, None);
    (counts.ht_active, counts.ht_secs) = calc_helltide(None);
}

fn view(counts: &Counts) -> Element<'_, Message> {
    let tsize = 30;
    let label_col = color!(0xffffff);
    let tcolor = color!(0x000000);

    let mut label_column = column![
        container(text("World Boss").size(tsize).color(label_col)).padding(1),
        container(text("Legion Event").size(tsize).color(label_col)).padding(1),
        container(text("Helltide").size(tsize).color(label_col)).padding(1),
    ];

    let mut val_column = column![
        container(text(get_hms(counts.wb)).size(tsize).color(tcolor))
            .style(move |_| container::background(get_color(counts.wb)))
            .padding(1),
        container(text(get_hms(counts.le)).size(tsize).color(tcolor))
            .style(move |_| container::background(get_color(counts.le)))
            .padding(1),
        container(
            text(get_helltide_str(counts.ht_active, counts.ht_secs))
                .size(tsize)
                .color(tcolor)
        )
        .style(move |_| container::background(get_helltide_color(
            counts.ht_active,
            counts.ht_secs
        )))
        .padding(1),
    ];

    if counts.realm_walker {
        label_column = label_column.push(
            container(text("Realm Walker").size(tsize).color(label_col))
                .padding(1),
        );
        val_column = val_column.push(
            container(text(get_hms(counts.rw)).size(tsize).color(tcolor))
                .style(move |_| container::background(get_color(counts.rw)))
                .padding(1),
        );
    }

    if counts.asmodan {
        label_column = label_column.push(
            container(text("Assmodan").size(tsize).color(label_col)).padding(1),
        );
        val_column = val_column.push(
            container(text(get_hms(counts.ass)).size(tsize).color(tcolor))
                .style(move |_| container::background(get_color(counts.ass)))
                .padding(1),
        );
    }

    let cont = container(row![label_column, val_column].spacing(10))
        .style(move |_| container::background(color!(0x2b2d31)))
        .padding(10);

    return cont.into();
}

fn subscription(_: &Counts) -> Subscription<Message> {
    time::every(std::time::Duration::from_secs(1)).map(|_| Message::Tick)
}

fn main() {
    let args = get_args();
    setup_logging(&args);

    // Base size: helltide + world boss + legion event (3 rows)
    let mut wsize = iced::Size::new(367.0, 150.0);
    if args.realm_walker {
        wsize.height += 40.0;
    }
    if args.asmodan {
        wsize.height += 40.0;
    }

    let _ = iced::application(
        move || Counts::new(args.asmodan, args.realm_walker),
        update,
        view,
    )
    .window_size(wsize)
    .title("Diablo 4 Events")
    .subscription(subscription)
    .run();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_hms() {
        let mut delta = 3600 + 8 * 60 + 34; // 1:08:34
        assert_eq!("  1:08:34  ".to_string(), get_hms(delta));

        delta = 3599; // 0:59:59
        assert_eq!("  0:59:59  ".to_string(), get_hms(delta));

        delta = 1; // 0:00:01
        assert_eq!("  0:00:01  ".to_string(), get_hms(delta));
    }

    #[test]
    fn test_calc_delta() {
        // init + 4:40:08
        let wb_ts = WB_INIT + 3600 * 4 + 40 * 60 + 8;
        assert_eq!(
            WB_EVERY - (3600 + 600 + 8),
            calc_delta(EventType::WB, Some(wb_ts))
        );

        // init + 1:10:08
        let le_ts = LE_INIT + 3600 + 600 + 8;
        assert_eq!(
            LE_EVERY - (20 * 60 + 8),
            calc_delta(EventType::LE, Some(le_ts))
        );

        // init + 1:10:08
        let rw_ts = RW_INIT + 3600 + 600 + 8;
        assert_eq!(
            RW_EVERY - (10 * 60 + 8),
            calc_delta(EventType::RW, Some(rw_ts))
        );
    }

    #[test]
    fn test_calc_helltide() {
        // Top of hour — helltide just started, 55:00 remaining
        let ts = 3600u64 * 100; // any exact hour boundary
        let (active, secs) = calc_helltide(Some(ts));
        assert!(active);
        assert_eq!(secs, HT_DURATION);

        // 1 second in — 54:59 remaining
        let (active, secs) = calc_helltide(Some(ts + 1));
        assert!(active);
        assert_eq!(secs, HT_DURATION - 1);

        // Last second of helltide (secs_in_hour = HT_DURATION - 1)
        let (active, secs) = calc_helltide(Some(ts + HT_DURATION - 1));
        assert!(active);
        assert_eq!(secs, 1);

        // Helltide just ended — 5:00 gap
        let (active, secs) = calc_helltide(Some(ts + HT_DURATION));
        assert!(!active);
        assert_eq!(secs, 3600 - HT_DURATION);

        // Last second before next helltide
        let (active, secs) = calc_helltide(Some(ts + 3599));
        assert!(!active);
        assert_eq!(secs, 1);
    }

    #[test]
    fn test_get_helltide_str() {
        assert_eq!("    -55:00  ", get_helltide_str(true, 3300));
        assert_eq!("    -00:01  ", get_helltide_str(true, 1));
        assert_eq!("    -00:00  ", get_helltide_str(true, 0));
        assert_eq!("  0:05:00  ", get_helltide_str(false, 300));
        assert_eq!("  0:00:01  ", get_helltide_str(false, 1));
    }
}
