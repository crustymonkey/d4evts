#[macro_use] extern crate log;
#[macro_use] extern crate clap;
#[macro_use] extern crate iced;

use chrono;
use clap::Parser;
use iced::widget::{text, column, container, row};
use iced::{time, Element, Subscription};
use std::time::{SystemTime, UNIX_EPOCH};

const APP_ID: &str = "com.splitstreams.d4evts";

#[derive(Parser, Debug)]
#[command(
    name=crate_name!(),
    author=crate_authors!(),
    version=crate_version!(),
    about=crate_description!(),
    long_about=None)
]
struct Args {
    /// Turn on debug output
    #[arg(short = 'D', long)]
    debug: bool,
}

static LOGGER: GlobalLogger = GlobalLogger;
// Setup the constants used for calculations
// World Boss
const WB_INIT: u64 = 1708381800;
const WB_EVERY: u64 = 60 * 210; // Every 3.5 hours
// Legion Event
const LE_INIT: u64 = 1708381200;
const LE_EVERY: u64 = 60 * 25; // Every 25 minutes
// Realm Walker
const RW_INIT: u64 = 1728414300;
const RW_EVERY: u64 = 60 * 15; // Every 15 minutes

/// This is used as an argument to calculate particular deltas
enum EventType {
    WB,
    LE,
    RW,
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
}

struct GlobalLogger;

/// This implements the logging to stderr from the `log` crate
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

/// Create a set of CLI args via the `clap` crate and return the matches
fn get_args() -> Args {
    return Args::parse();
}

/// Set the global logger from the `log` crate
fn setup_logging(args: &Args) {
    let l = if args.debug {
        log::LevelFilter::Debug
    } else {
        log::LevelFilter::Info
    };

    log::set_logger(&LOGGER).unwrap();
    log::set_max_level(l);
}

/// This calculates the time to the next event for the given EventType.
/// By default, it uses the current time for this calculation.  This returns
/// the time in seconds until the next World Boss or Legion Event
fn calc_delta(ev: EventType, ts: Option<u64>) -> u64 {
    // The ts argument here is for testing purposes
    let now = match ts {
        Some(t) => t,
        None => SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    };
    let elapsed;
    let delta;

    match ev {
        EventType::WB => {
            elapsed = (now - WB_INIT) % WB_EVERY;
            delta = WB_EVERY - elapsed;
        }
        EventType::LE => {
            elapsed = (now - LE_INIT) % LE_EVERY;
            delta = LE_EVERY - elapsed;
        }
        EventType::RW => {
            elapsed = (now - RW_INIT) % RW_EVERY;
            delta = RW_EVERY - elapsed;
        }
    };

    debug!("Got delta of: {delta}");
    return delta;
}

/// This converts a delta (time remaining until event) in the form of seconds
/// into an H:MM:SS string
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

fn get_color(delta: u64) -> iced::Color {
    if delta <= 300 {
        return color!(0xff8080); // Red
    } else if delta <= 600 {
        return color!(0xffff00); // Yellow
    } else {
        return color!(0x808080); // Gray
    }
}

fn update(counts: &mut Counts, _: Message) {
    counts.wb = calc_delta(EventType::WB, None);
    counts.le = calc_delta(EventType::LE, None);
    counts.rw = calc_delta(EventType::RW, None);
}

/// This actually builds out the UI and presents it.  It also spawns the
/// future loop
fn view(counts: &Counts) -> Element<'_, Message> {
    let tsize = 30;
    let label_col = color!(0xffffff);
    let tcolor = color!(0x000000);
    let cont = container(
        row![
            column![
                container(text("World Boss").size(tsize).color(label_col))
                    .padding(1),
                container(text("Legion Event").size(tsize).color(label_col))
                    .padding(1),
                container(text("Realm Walker").size(tsize).color(label_col))
                    .padding(1),
            ],
            column![
                container(text(get_hms(counts.wb)).size(tsize).color(tcolor))
                    .style(move |_| container::background(get_color(counts.wb)))
                    .padding(1),
                container(text(get_hms(counts.le)).size(tsize).color(tcolor))
                    .style(move |_| container::background(get_color(counts.le)))
                    .padding(1),
                container(text(get_hms(counts.rw)).size(tsize).color(tcolor))
                    .style(move |_| container::background(get_color(counts.rw)))
                    .padding(1),

            ]
        ]
        .spacing(10)
    )
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

    let _ = iced::application(Counts::default, update, view)
        .window_size(iced::Size::new(367.0, 150.0))
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
        ); // 1:10:08

        // init + 1:10:08
        let le_ts = LE_INIT + 3600 + 600 + 8;
        assert_eq!(
            LE_EVERY - (20 * 60 + 8),
            calc_delta(EventType::LE, Some(le_ts))
        ); // 0:20:08

        // init + 1:10:08
        let rw_ts = RW_INIT + 3600 + 600 + 8;
        assert_eq!(
            RW_EVERY - (10 * 60 + 8),
            calc_delta(EventType::RW, Some(rw_ts))
        ); // 0:20:08
    }
}
