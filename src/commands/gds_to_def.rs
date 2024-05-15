use std::any::Any;
use std::fs::File;
use std::io::BufReader;
use std::ops::{Add, Sub};
use std::path::PathBuf;

use ::libreda_db::chip::Chip;
use ::libreda_db::prelude as db;
use ::libreda_db::prelude::*;
use gds21::{GdsBoundary, GdsDateTimes, GdsElement, GdsLibrary, GdsPath, GdsPoint, GdsProperty, GdsStrans, GdsStruct, GdsStructRef, GdsTextElem, GdsUnits};
use iron_shapes::concepts::Rectangle;
use iron_shapes::prelude::*;
use iron_shapes::prelude::Orientation2D::Horizontal;
use libreda_db::prelude::FnName::{each_cell_instance, template_cell};
use libreda_lefdef::import::{DEFImportOptions, import_def_into_db, import_lef_into_db, LEFImportOptions};
use libreda_oasis::OASISStreamWriter;
use uuid::Uuid;

/// A trait to constrain coordinate types used in shapes, ensuring they implement required traits.
pub trait CoordConstraints: CoordinateType + std::fmt::Debug + std::fmt::Display {}

impl<T> CoordConstraints for T where T: CoordinateType + std::fmt::Debug + std::fmt::Display {}

/// Convert a DEF file to a GDSII layout.
///
/// # Arguments
/// * `_top` - Top-level cell name (currently unused)
/// * `input` - Path to the DEF file
/// * `output` - Output file path
/// * `lef_files` - List of LEF files to import
///
/// # Returns
/// A result indicating the success or failure of the conversion process.
pub fn convert_def_to_gds(
    _top: &str,
    input: &PathBuf,
    output: &PathBuf,
    lef_files: &[&PathBuf],
) -> Result<bool, Box<dyn std::error::Error>> {
    // Create a GDS-to-DEF conversion flow.
    let mut flow: GdsToDefFlow<Chip> = GdsToDefFlow::new();

    // Import LEF files into the database.
    flow.import_lefs_into_db(&lef_files);

    // Import the DEF design into the DB format.
    flow.import_gds_into_db(&input);

    dbg!(&flow.chip);
    // Export the design into the GDSII format.
    flow.generate_def_file(&output);

    Ok(true)
}


/// Struct representing the conversion flow between GDS and DEF files.
#[derive(Clone)]
pub struct GdsToDefFlow<C>
    where
        C: db::L2NEdit,
{
    /// Layout and netlist data.
    pub chip: C,
    /// Path to technology LEF file.
    pub tech_lef_path: std::path::PathBuf,
    /// Technology LEF data.
    pub tech_lef: libreda_lefdef::LEF,
    /// Top cell identifier.
    pub top_cell: Option<C::CellId>,
    /// DEF data.
    pub def: libreda_lefdef::DEF,
    /// Names of clock nets in the top cell.
    pub clock_nets: Vec<String>,
    /// Names of reset nets in the top cell.
    pub reset_nets: Vec<String>,
    /// The shape of the chip/macro.
    pub core_area: Option<db::SimplePolygon<C::Coord>>,
    /// The layer to be used for cell outlines (abutment boxes).
    pub outline_layer: Option<C::LayerId>,
    /// Target density for placement. Must be in the range `(0.0, 1.0]`.
    pub placement_target_density: f64,
}

impl<C> GdsToDefFlow<C>
    where
        C: db::L2NEdit<Coord=db::Coord> + L2NBase + Default,
{
    /// Create a new `GdsToDefFlow` instance.
    pub fn new() -> Self {
        let mut simple_flow = Self {
            chip: Default::default(),
            tech_lef_path: Default::default(),
            tech_lef: Default::default(),
            top_cell: Default::default(),
            def: Default::default(),
            clock_nets: Default::default(),
            reset_nets: Default::default(),
            core_area: Default::default(),
            outline_layer: Default::default(),
            placement_target_density: 0.5,
        };

        simple_flow.init();
        simple_flow
    }

    /// Initialize the conversion flow.
    fn init(&mut self) {
        // Set database units to 1000 (DBU).
        self.chip.set_dbu(1000);
    }

    /// Import LEF files into the database.
    pub fn import_lefs_into_db(&mut self, lef_files: &[&PathBuf]) {
        self.tech_lef_path = lef_files.get(0).expect("Expected at least one LEF").into();

        let lef_contents: Vec<_> = lef_files.iter().map(|f| self.import_lef(f)).collect();

        // Import each LEF library into the database format.
        for lef in lef_contents {
            let options = LEFImportOptions::default();
            self.tech_lef = lef.clone(); // TODO : overrides but there be more than one lef
            import_lef_into_db(&options, &lef, &mut self.chip).expect("Failed to import LEF.");
        }
    }

    /// Import a single LEF file.
    fn import_lef(&self, fp: &PathBuf) -> libreda_lefdef::LEF {
        let fh = File::open(fp).expect("Failed to open LEF file.");
        let mut buf = BufReader::new(fh);

        let result = libreda_lefdef::lef_parser::read_lef_bytes(&mut buf);

        result.expect("Failed to parse LEF.")
    }

    /// Imports GDS data into the database format.
    pub fn import_gds_into_db(&mut self, gds_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        let file = File::open(gds_path)?;
        let mut reader = BufReader::new(file);
        let gds_lib = GdsLibrary::read_from(&mut reader)?;

        for gds_struct in gds_lib.structs {
            self.process_gds_struct(&gds_struct)?;
        }

        Ok(())
    }
}