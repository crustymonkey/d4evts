#[macro_use]
extern crate log;
#[macro_use]
extern crate clap;

use chrono;
use clap::Parser;
use eframe::egui::{self, Color32, RichText};
use std::{
    sync::{Arc, Mutex},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

#[derive(Parser, Debug)]
#[command(
    name=crate_name!(),
    author=crate_authors!(),
    version=crate_version!(),
    about=crate_description!(),
    long_about=None)
]
struct Args {
    /// Set the font size
    #[arg(short, long, default_value_t = 30.0)]
    font_size: f32,
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

/// This is a struct that contains the core GUI app data
struct MyApp {
    wb: Arc<Mutex<String>>,
    wb_color: Arc<Mutex<Color32>>,
    le: Arc<Mutex<String>>,
    le_color: Arc<Mutex<Color32>>,
    rw: Arc<Mutex<String>>,
    rw_color: Arc<Mutex<Color32>>,
    font_size: f32,
}

impl MyApp {
    /// Pass in the font size to create a new app object
    fn new(font_size: f32) -> Self {
        return Self {
            wb: Arc::new(Mutex::new("".to_string())),
            wb_color: Arc::new(Mutex::new(Color32::GRAY)),
            le: Arc::new(Mutex::new("".to_string())),
            le_color: Arc::new(Mutex::new(Color32::GRAY)),
            rw: Arc::new(Mutex::new("".to_string())),
            rw_color: Arc::new(Mutex::new(Color32::GRAY)),
            font_size: font_size,
        };
    }
}

impl Default for MyApp {
    /// Create an app object using defaults
    fn default() -> Self {
        return Self {
            wb: Arc::new(Mutex::new("".to_string())),
            wb_color: Arc::new(Mutex::new(Color32::GRAY)),
            le: Arc::new(Mutex::new("".to_string())),
            le_color: Arc::new(Mutex::new(Color32::GRAY)),
            rw: Arc::new(Mutex::new("".to_string())),
            rw_color: Arc::new(Mutex::new(Color32::GRAY)),
            font_size: 30.0,
        };
    }
}

impl eframe::App for MyApp {
    /// This sets up the design of the GUI
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint();
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::Grid::new("some ID").show(ui, |ui| {
                ui.label(RichText::new("World Boss").size(self.font_size).strong());
                ui.label(
                    RichText::new(format!("{}", self.wb.lock().unwrap()))
                        .size(self.font_size)
                        .color(Color32::BLACK)
                        .background_color(*self.wb_color.lock().unwrap()),
                );

                ui.end_row();
                ui.label(RichText::new("Legion Event").size(self.font_size).strong());
                ui.label(
                    RichText::new(format!("{}", self.le.lock().unwrap()))
                        .size(self.font_size)
                        .color(Color32::BLACK)
                        .background_color(*self.le_color.lock().unwrap()),
                );
                ui.end_row();
                ui.label(RichText::new("Realm Walker").size(self.font_size).strong());
                ui.label(
                    RichText::new(format!("{}", self.rw.lock().unwrap()))
                        .size(self.font_size)
                        .color(Color32::BLACK)
                        .background_color(*self.rw_color.lock().unwrap()),
                );
                ui.end_row();
            });
        });
    }
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
        },
        EventType::RW => {
            elapsed = (now - RW_INIT) % RW_EVERY;
            delta = RW_EVERY - elapsed;
        },
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

/// This runs the main update loop.  This will update the countdown clock and
/// their background colors.
fn run_update_thread(
    wb: Arc<Mutex<String>>,
    wb_color: Arc<Mutex<Color32>>,
    le: Arc<Mutex<String>>,
    le_color: Arc<Mutex<Color32>>,
    rw: Arc<Mutex<String>>,
    rw_color: Arc<Mutex<Color32>>,
) {
    loop {
        thread::sleep(Duration::from_secs(1));
        let wb_delta = calc_delta(EventType::WB, None);
        let le_delta = calc_delta(EventType::LE, None);
        let rw_delta = calc_delta(EventType::RW, None);

        *wb.lock().unwrap() = get_hms(wb_delta);
        if wb_delta <= 300 {
            *wb_color.lock().unwrap() = Color32::LIGHT_RED;
        } else {
            *wb_color.lock().unwrap() = Color32::GRAY;
        }

        *le.lock().unwrap() = get_hms(le_delta);
        if le_delta <= 300 {
            *le_color.lock().unwrap() = Color32::LIGHT_RED;
        } else {
            *le_color.lock().unwrap() = Color32::GRAY;
        }
        *rw.lock().unwrap() = get_hms(rw_delta);
        if rw_delta <= 300 {
            *rw_color.lock().unwrap() = Color32::LIGHT_RED;
        } else {
            *rw_color.lock().unwrap() = Color32::GRAY;
        }
    }
}

fn main() {
    let args = get_args();
    setup_logging(&args);

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([340.0, 140.0]),
        ..Default::default()
    };

    let app = MyApp::new(args.font_size);

    let wb = app.wb.clone();
    let wb_color = app.wb_color.clone();
    let le = app.le.clone();
    let le_color = app.le_color.clone();
    let rw = app.rw.clone();
    let rw_color = app.rw_color.clone();

    thread::spawn(move || {
        run_update_thread(wb, wb_color, le, le_color, rw, rw_color);
    });

    eframe::run_native(
        "Diablo 4 Events",
        options,
        Box::new(move |_cc| Box::new(app)),
    )
    .expect("Failed to start the gui");
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
