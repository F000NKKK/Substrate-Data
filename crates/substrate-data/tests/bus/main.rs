//! Data as a message bus carries it: written and read straight (no value
//! tree), streams, compact owned types, frames packed by position, views.

mod compact;
mod frames;
mod messages;
mod streams;
mod views;

use compact::Compact;
use messages::{Message, Mode};
use streams::{Entry, Folder};
