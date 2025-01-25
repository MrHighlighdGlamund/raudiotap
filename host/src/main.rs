use nih_plug::prelude::*;
use raudiotap::Raudiotap;

fn main() {
    nih_export_standalone::<Raudiotap>();
}
