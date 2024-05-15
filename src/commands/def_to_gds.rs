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
    let mut flow: DefToGdsFlow<Chip> = DefToGdsFlow::new();

    // Import LEF files into the database.
    flow.import_lefs_into_db(&lef_files);

    // Import the DEF design into the DB format.
    flow.import_def_into_db(&input);

    dbg!(&flow.chip);
    // Export the design into the GDSII format.
    flow.generate_gds_file(&output);

    Ok(true)
}

/// Convert a DEF file to a OASIS layout.
///
/// # Arguments
/// * `_top` - Top-level cell name (currently unused)
/// * `input` - Path to the DEF file
/// * `output` - Output file path
/// * `lef_files` - List of LEF files to import
///
/// # Returns
/// A result indicating the success or failure of the conversion process.
pub fn convert_def_to_oasis(
    _top: &str,
    input: &PathBuf,
    output: &PathBuf,
    lef_files: &[&PathBuf],
) -> Result<bool, Box<dyn std::error::Error>> {
    // Create a GDS-to-DEF conversion flow.
    let mut flow: DefToGdsFlow<Chip> = DefToGdsFlow::new();

    // Import LEF files into the database.
    flow.import_lefs_into_db(&lef_files);

    // Import the DEF design into the DB format.
    flow.import_def_into_db(&input);

    // Export the design into the GDSII format.
    flow.generate_oasis_file(&output);

    Ok(true)
}

/// Struct representing the conversion flow between GDS and DEF files.
#[derive(Clone)]
pub struct DefToGdsFlow<C>
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

impl<C> DefToGdsFlow<C>
    where
        C: db::L2NEdit<Coord=db::Coord> + L2NBase + Default,
{
    /// Create a new `DefToGdsFlow` instance.
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

    /// Import the DEF file into the database.
    pub fn import_def_into_db(&mut self, input: &PathBuf) {
        let def = self.import_def(input);
        let def_import_options: DEFImportOptions<_> = DEFImportOptions::default();

        import_def_into_db(&def_import_options, Some(&self.tech_lef), &def, &mut self.chip)
            .expect("Failed to import DEF.");
        let name = def.design_name.expect("DEF error, design name expected");
        self.top_cell = Some(self.chip
            .cell_by_name(name.as_str())
            .expect("DEF error, design name not found"));
    }

    /// Import a single DEF file.
    fn import_def(&self, fp: &PathBuf) -> libreda_lefdef::DEF {
        let fh = File::open(fp).expect("Failed to open DEF file.");
        let mut buf = BufReader::new(fh);

        let result = libreda_lefdef::def_parser::read_def_bytes(&mut buf);

        result.expect("Failed to parse DEF.")
    }

    /// Generate a GDS file from the chip data using OASIS.
    pub fn generate_gds_file(self, fp: &PathBuf) {
        // let mut fh = File::create(fp).expect("Failed to create GDS file.");
        // let writer = OASISStreamWriter::default();
        // writer.write_layout(&mut fh, &self.chip).expect("Failed to write GDS layout.");
        self._generate_gds_file_with_gds21(&fp);
    }

    /// Generate an OASIS file from the chip data.
    pub fn generate_oasis_file(&self, fp: &PathBuf) {
        let mut oasis_path = fp.clone();
        oasis_path.set_extension("oas");

        let mut fh = File::create(&oasis_path).expect("Failed to create OASIS file");

        let writer = OASISStreamWriter::default();
        writer
            .write_layout(&mut fh, &self.chip)
            .expect("Failed to write OASIS layout");
    }

    /// Generate a GDS file using gds21.
    fn _generate_gds_file_with_gds21(self, fp: &PathBuf) {
        let mut gds_path = fp.clone();
        gds_path.set_extension("gds");
        let top_cell = self.top_cell;
        let gds_library: GdsLibrary = chip_to_gds_library(&self.chip, top_cell.unwrap().into());

        gds_library
            .save(&gds_path)
            .expect("Failed to write GDS layout using gds21");
    }
}


/// Convert a `libreda_db::chip::Chip` to a `GdsLibrary`.
///
/// # Arguments
/// * `chip` - The chip layout to convert.
/// * `top`  - The top level cell id.
///
/// # Returns
/// A `GdsLibrary` representing the chip.
pub fn chip_to_gds_library<C: L2NBase>(chip: &C, top: C::CellId) -> GdsLibrary
    where
        C::Coord: Into<i32>,
{
    let design_name = chip.cell_name(&top);
    let mut gds_library = GdsLibrary {
        name: design_name.to_string(),
        version: 5,
        dates: GdsDateTimes::default(),
        units: GdsUnits::default(),
        structs: vec![],
        ..Default::default()
    };

    let mut top_cell_struct = GdsStruct {
        name: design_name.to_string(),
        dates: Default::default(),
        elems: vec![],
    };

    let boundary_layer = chip.layer_by_name("OUTLINE").unwrap();
    let boundary_shapes = chip.each_shape_id(&top, &boundary_layer).count();
    assert_eq!(boundary_shapes, 1);
    // boundary_shapes .for_each(|s| {
    //     let layer_info = chip.layer_info(&chip.shape_layer(&s));
    //     top_cell_struct.elems.push(
    //         shape_to_gds_element(
    //             &chip.shape_geometry(&s),
    //             layer_info.index as i16,
    //             vec![]
    //         )
    //     )
    // });

    chip.each_shape_id(&top, &boundary_layer)
        .for_each(|s: C::ShapeId| {
            let layer_info = chip.layer_info(&chip.shape_layer(&s));
            top_cell_struct.elems.push(
                shape_to_gds_element(
                    &chip.shape_geometry(&s),
                    layer_info.index as i16,
                    vec![]
                )
            )
        });

    for cell in chip.each_cell() {
        if chip.num_cell_dependencies(&cell) != 0 || chip.num_dependent_cells(&cell) != 0 && cell != top {
            gds_library.structs.push(chip_cell_to_gds_struct(chip, &cell));
        }
    }

    chip_cell_to_gds_struct(chip, &top);
    chip.each_internal_net(&top)
        .filter(|n| !chip.is_constant_net(&n))
        .for_each(|n: C::NetId| {
            chip.shapes_of_net(&n)
                .for_each(|s| {
                    let layer_info = chip.layer_info(&chip.shape_layer(&s));
                    top_cell_struct.elems.push(
                        shape_to_gds_element(
                            &chip.shape_geometry(&s),
                            layer_info.index as i16,
                            // vec!(
                            //     GdsProperty{
                            //         attr: 1,
                            //         value: chip.get_chip_property(
                            //
                            //         )
                            //     }
                            // )
                            vec![]
                        )
                    )
                })
        });

    // nets.eac
    gds_library
}


/// Convert a `libreda_db::chip::Cell` to a `GdsStruct`.
///
/// # Arguments
/// * `chip` - The chip containing the cell.
/// * `cell` - The cell to convert.
///
/// # Returns
/// A `GdsStruct` representing the cell.
pub fn chip_cell_to_gds_struct<C: LayoutBase>(layout: &C, cell: &C::CellId) -> GdsStruct
    where
        C::Coord: Into<i32>
{
    let cell_name = {
        let it = layout.cell_name(cell).into();
        if it.is_empty() {
            "Unnamed Cell".to_string()
        } else {
            it
        }
    };

    let mut gds_struct = GdsStruct {
        name: cell_name,
        dates: GdsDateTimes::default(),
        elems: vec![],
    };

    for layer in layout.each_layer() {
        let layer_info = layout.layer_info(&layer);
        layout.for_each_shape(cell, &layer, |_, shape| {
            gds_struct.elems.push(shape_to_gds_element(shape, layer_info.index as i16, vec![]));
        });
    }

    layout.for_each_cell_instance(&cell, |inst| {
        // Write PLACEMENT records.
        let placement_cell = layout.template_cell(&inst);

        let tf = layout.get_transform(&inst);
        let (new_x, new_y, new_rotation, should_flip) = layout.bounding_box(&placement_cell)
            .map(|bbox| {
                let width: i32 = (bbox.upper_right().x - bbox.lower_left().x).into();
                let height: i32 = (bbox.upper_right().y - bbox.lower_left().y).into();

                // Calculate new origin based on rotation
                let (tf_x, tf_y, new_rotation, should_flip) = match (tf.mirror, tf.rotation) {
                    (false, Angle::R0) => (0, 0, Angle::R0, false),
                    (false, Angle::R180) => (width, height, Angle::R180, false),
                    (false, Angle::R90) => (height, 0, Angle::R90, false),
                    (false, Angle::R270) => (0, width, Angle::R270, false),
                    (true, Angle::R0) => (width, 0, Angle::R180, true),
                    (true, Angle::R180) => (0, height, Angle::R0, true),
                    (true, Angle::R90) => (0, 0, Angle::R270, true),
                    (true, Angle::R270) => (height, width, Angle::R90, true),
                };

                // Adjust xy for placement to have bottom-left corner at the displacement point
                let new_x = tf.displacement.x.into() + tf_x;
                let new_y = tf.displacement.y.into() + tf_y;
                (new_x, new_y, new_rotation, should_flip)
            }).unwrap();

        gds_struct.elems.push(GdsElement::GdsStructRef(
            GdsStructRef {
                name: layout.cell_name(&placement_cell).to_string(),
                xy: GdsPoint { x: new_x.into(), y: new_y.into() },
                strans: GdsStrans {
                    reflected: should_flip,
                    abs_mag: true,
                    mag: Some(tf.magnification.into() as f64),
                    abs_angle: false,
                    angle: Some(90.0 * (new_rotation.as_int() as f64)),
                }.into(),
                elflags: None,
                plex: None,
                properties: vec!(
                    GdsProperty {
                        attr: 1,
                        value: layout
                            .cell_instance_name(&inst)
                            .map(|n| n.into())
                            .unwrap_or_else(|| layout.cell_name(&placement_cell).to_string()),
                    }
                ),
            }
        ))
    });

    gds_struct
}

/// Convert a `Geometry` shape to a `GdsBoundary`.
///
/// # Arguments
/// * `shape` - The shape to convert.
/// * `layer_index` - The layer index to assign to the shape.
///
/// # Returns
/// A `GdsBoundary` representing the shape.
pub fn shape_to_gds_element<C>(shape: &Geometry<C>, layer_index: i16, properties: Vec<GdsProperty>) -> GdsElement
    where
        C: CoordConstraints + Into<i32> + Copy,
{
    match shape {
        Geometry::SimplePolygon(poly) => GdsElement::GdsBoundary(
            polygon_to_gds_element(poly, layer_index, 0i16, properties)
        ),
        Geometry::SimpleRPolygon(rpoly) => GdsElement::GdsBoundary(
            polygon_to_gds_element(&rpoly.to_simple_polygon(), layer_index, 0i16, properties)
        ),
        Geometry::Polygon(Polygon { exterior, .. }) => GdsElement::GdsBoundary(
            polygon_to_gds_element(exterior, layer_index, 0i16, properties)
        ),
        Geometry::Rect(rect) => GdsElement::GdsBoundary(
            rect_to_gds_element(rect, layer_index, 0i16, properties)
        ),
        Geometry::Path(path) => GdsElement::GdsPath(
            path_to_gds_path(path, layer_index, 0i16, properties)
        ),
        _ => {
            dbg!(shape);
            GdsElement::GdsTextElem(GdsTextElem {
                string: "UNKNOWN_SHAPE".to_string(),
                layer: 1,
                texttype: 0,
                xy: Default::default(),
                ..Default::default()
            })
        }
    }
}

/// Convert a `SimplePolygon` to a list of `gds21::GdsPoint`.
///
/// # Arguments
/// * `poly` - The polygon to convert.
///
/// # Returns
/// A `GdsBoundary` representing the polygon.
pub fn polygon_to_gds_element<C>(poly: &SimplePolygon<C>, layer_index: i16, data_type: i16, properties: Vec<GdsProperty>) -> GdsBoundary
    where
        C: Into<i32> + Copy,
{
    let mut points: Vec<GdsPoint> = poly
        .points()
        .iter()
        .map(|p| GdsPoint {
            x: p.x.into(),
            y: p.y.into(),
        })
        .collect();

    // Close the polygon by adding the first point again.
    if let Some(first) = points.first() {
        points.push(first.clone());
    }

    GdsBoundary {
        layer: layer_index,
        datatype: data_type,
        xy: points,
        ..Default::default()
    }
}

/// Convert a `Rect` to a list of `gds21::GdsPoint`.
///
/// # Arguments
/// * `rect` - The rectangle to convert.
///
/// # Returns
/// A `GdsBoundary` representing the rectangle.
pub fn rect_to_gds_element<C>(rect: &Rect<C>, layer_index: i16, data_type: i16, properties: Vec<GdsProperty>) -> GdsBoundary
    where
        C: Into<i32> + Copy,
{
    GdsBoundary {
        layer: layer_index,
        datatype: data_type,
        xy: vec![
            GdsPoint { x: rect.lower_left().x.into(), y: rect.lower_left().y.into() },
            GdsPoint { x: rect.lower_right().x.into(), y: rect.lower_right().y.into() },
            GdsPoint { x: rect.upper_right().x.into(), y: rect.upper_right().y.into() },
            GdsPoint { x: rect.upper_left().x.into(), y: rect.upper_left().y.into() },
            GdsPoint { x: rect.lower_left().x.into(), y: rect.lower_left().y.into() },
        ],
        ..Default::default()
    }
}

/// Convert a `Path` to a `GdsPath`.
///
/// # Arguments
/// * `path` - The path to convert.
/// * `layer_index` - The layer index to assign to the path.
/// * `datatype` - The datatype ID to assign to the path.
///
/// # Returns
/// A `GdsPath` representing the path with specified attributes.
pub fn path_to_gds_path<C>(path: &Path<C>, layer_index: i16, datatype: i16, properties: Vec<GdsProperty>) -> GdsPath
    where
        C: Into<i32> + Copy,
{
    let points: Vec<GdsPoint> = path.points.iter().map(|p| GdsPoint {
        x: p.x.into(),
        y: p.y.into(),
    }).collect();

    //   Type of path endpoints (int, optional). The values have the following meaning:
    //       0 – square ends, flush with endpoints
    //       1 – round ends, centered on endpoints
    //       2 – square ends, centered on endpoints
    //       4 – custom square ends
    let (path_type, begin_extn, end_extn) = match path.path_type {
        PathEndType::Flat => (Some(0), None, None),
        PathEndType::Extended(b, e) => (Some(2), Some(b.into()), Some(e.into())),
        PathEndType::Round => (Some(1), None, None)
    };

    GdsPath {
        layer: layer_index,
        datatype: datatype,
        xy: points,
        width: Some(path.width.into()),
        path_type: path_type,
        begin_extn: begin_extn,
        end_extn: end_extn,
        properties: properties,
        ..Default::default()
    }
}
