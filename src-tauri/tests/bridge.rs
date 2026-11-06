// Compile The Native Commands In Tests As The Library Excludes Desktop Under Cfg Test.
#[allow(dead_code)]
#[path = "../src/files.rs"]
mod files;
#[allow(dead_code)]
#[path = "../src/recent.rs"]
mod recent;
#[allow(dead_code)]
#[path = "../src/sessions.rs"]
mod sessions;
#[allow(dead_code)]
#[path = "../src/statistics.rs"]
mod statistics;
#[allow(dead_code)]
#[path = "../src/desktop.rs"]
mod desktop;

#[allow(dead_code)]
#[path = "../src/links.rs"]
mod links;

#[allow(dead_code)]
#[path = "../src/startup.rs"]
mod startup;

#[allow(dead_code)]
#[path = "../src/downloads.rs"]
mod downloads;

#[allow(dead_code)]
#[path = "../src/pdf.rs"]
mod pdf;
