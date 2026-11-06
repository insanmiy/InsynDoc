mod files;
mod sessions;
mod statistics;
mod recent;
mod links;
mod startup;
mod downloads;

#[cfg(not(test))]
mod desktop;

#[cfg(not(test))]
mod pdf;

#[cfg(not(test))]
pub use desktop::run;
