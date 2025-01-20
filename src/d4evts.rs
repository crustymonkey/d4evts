#[macro_use]
extern crate log;
#[macro_use]
extern crate clap;

use chrono;
use clap::Parser;
use gtk4::{gdk::Display, glib, prelude::*, Application, ApplicationWindow};
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

/// This updates the actual label widgets with the calculated deltas
fn update_view(wb: &gtk4::Label, le: &gtk4::Label, rw: &gtk4::Label) {
    let wb_delta = calc_delta(EventType::WB, None);
    let le_delta = calc_delta(EventType::LE, None);
    let rw_delta = calc_delta(EventType::RW, None);

    wb.set_label(get_hms(wb_delta).as_str());
    if wb_delta <= 300 {
        wb.set_css_classes(&["danger"]);
    } else if wb_delta <= 600 {
        wb.set_css_classes(&["warning"]);
    } else {
        wb.set_css_classes(&["normal"]);
    }

    le.set_label(get_hms(le_delta).as_str());
    if le_delta <= 300 {
        le.set_css_classes(&["danger"]);
    } else if le_delta <= 600 {
        le.set_css_classes(&["warning"]);
    } else {
        le.set_css_classes(&["normal"]);
    }

    rw.set_label(get_hms(rw_delta).as_str());
    if rw_delta <= 300 {
        rw.set_css_classes(&["danger"]);
    } else if rw_delta <= 600 {
        rw.set_css_classes(&["warning"]);
    } else {
        rw.set_css_classes(&["normal"]);
    }
}

/// This will load the CSS from the style.css file for the widget styling
fn load_css() {
    let provider = gtk4::CssProvider::new();
    provider.load_from_string(include_str!("../style.css"));

    gtk4::style_context_add_provider_for_display(
        &Display::default().expect("Failed to connect to a display"),
        &provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

/// This actually builds out the UI and presents it.  It also spawns the
/// future loop
fn build_ui(app: &Application) {
    let hzt_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .build();

    let name_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .build();

    let time_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .build();

    let wb_name = gtk4::Label::builder()
        .label("World Boss")
        .halign(gtk4::Align::Start)
        .build();
    wb_name.add_css_class("base");

    let le_name = gtk4::Label::builder()
        .label("Legion Event")
        .halign(gtk4::Align::Start)
        .build();
    le_name.add_css_class("base");

    let rw_name = gtk4::Label::builder()
        .label("Realm Walker")
        .halign(gtk4::Align::Start)
        .build();
    rw_name.add_css_class("base");

    let wb = gtk4::Label::builder().label("0").build();
    let le = gtk4::Label::builder().label("0").build();
    let rw = gtk4::Label::builder().label("0").build();

    wb.add_css_class("normal");
    le.add_css_class("normal");
    rw.add_css_class("normal");

    // Run the update thread
    glib::spawn_future_local(glib::clone!(
        #[weak]
        wb,
        #[weak]
        le,
        #[weak]
        rw,
        async move {
            loop {
                update_view(&wb, &le, &rw);
                glib::timeout_future_seconds(1).await;
            }
        }
    ));

    // Build the UI arrangement
    hzt_box.append(&name_box);
    hzt_box.append(&time_box);
    name_box.append(&wb_name);
    name_box.append(&le_name);
    name_box.append(&rw_name);
    time_box.append(&wb);
    time_box.append(&le);
    time_box.append(&rw);

    // Create and display the window
    let window = ApplicationWindow::builder()
        .application(app)
        .title("Diablo 4 Events")
        .child(&hzt_box)
        .build();

    window.present()
}

fn main() {
    let args = get_args();
    setup_logging(&args);
    let app = Application::builder().application_id(APP_ID).build();

    app.connect_startup(|_| load_css());
    app.connect_activate(build_ui);
    app.run();
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
