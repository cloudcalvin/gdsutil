
//
// /// Serialize the `layout` to the `writer` in the OASIS format.
// /// The `conf` structure is used for controlling the output format.
// pub fn write_layout<W: Write, L: LayoutBase<Coord=SInt>>(
//     writer: &mut W,
//     layout: &L,
//     conf: &OASISWriterConfig,
// ) -> Result<(), dyn Error> {
//     trace!("Write layout to GDS");
//
//     let mut gds_library = GdsLibrary {
//         name: "Libreda Conversion".into(),
//         version: 5,
//         dates: GdsDateTimes::default(),
//         units: GdsUnits::default(),
//         structs: vec![],
//         ..Default::default()
//     };
//
//
//     // // Modal variables.
//     // let mut modal = Modal::default();
//     //
//     // write_magic(writer)?;
//     // // Write START record.
//     // write_unsigned_integer(writer, 1)?;
//     // // Write version string.
//     // write_ascii_string(writer, "1.0".as_bytes())?;
//     //
//     // // Write resolution.
//     // if layout.dbu().is_zero() {
//     //     return Err(OASISWriteError::DbuIsZero);
//     // }
//     // let resolution = Real::PositiveWholeNumber(layout.dbu() as u32);
//     // write_real(writer, resolution)?;
//     //
//     // // Put offset table at start or at end?
//     // let offset_flag = if conf.table_offsets_at_start { 0 } else { 1 };
//     // write_unsigned_integer(writer, offset_flag)?;
//     // if conf.table_offsets_at_start {
//     //     // table-offsets is stored in START record.
//     //     write_offset_table(writer, &OffsetTable::default())?; // TODO: Construct proper offset table.
//     // }
//     //
//     // // Define counters for implicit IDs.
//     // // TODO: Writing implicit IDs is not supported yet.
//     // let mut _cellname_id_counter = (0..).into_iter();
//     // let mut _textstrings_id_counter = (0..).into_iter();
//     // let mut _propname_id_counter = (0..).into_iter();
//     // let mut _propstrings_id_counter = (0..).into_iter();
//
//     // Loop through all cells.
//     for cell in layout.each_cell() {
//         let cell_name_raw = layout.cell_name(&cell);
//         let cell_name: &str = cell_name_raw.borrow();
//         trace!("write cell '{:?}'", cell_name);
//
//         // Write CELL record (identified by name).
//         // TODO: Support CELL record 13 by ID.
//         // write_unsigned_integer(writer, 14)?;
//         // write_name_string(writer, cell_name.as_bytes())?;
//         //
//         // modal.reset();
//         let gds_struct = chip_cell_to_gds_struct(layout, &cell);
//         gds_library.structs.push(gds_struct);
//
//         // Write all shapes inside this cell.
//         for layer_id in layout.each_layer() {
//             trace!("write shapes on layer '{:?}'", layer_id);
//             let layer_info = layout.layer_info(&layer_id);
//             let layer_index = layer_info.index;
//             let layer_datatype = layer_info.datatype;
//
//             // Clone all shapes.
//             // TODO: This is a hack around the current layout trait. Find a solution where geometries need not be cloned.
//             let mut shapes = Vec::new();
//             layout.for_each_shape(&cell, &layer_id, |_id, geo| shapes.push(geo.clone()));
//
//             for geometry in &shapes {
//                 match geometry {
//                     Geometry::Rect(r) => {
//                         write_rect(writer, &mut modal, r, layer_index, layer_datatype)?
//                     }
//                     Geometry::SimplePolygon(p) => {
//                         write_simple_polygon(writer, &mut modal, p, layer_index, layer_datatype)?
//                     }
//                     Geometry::SimpleRPolygon(p) =>
//                     // TODO: Don't convert to generic polygon but write a manhattanized polygon. This saves space.
//                         {
//                             write_simple_polygon(
//                                 writer,
//                                 &mut modal,
//                                 &p.to_simple_polygon(),
//                                 layer_index,
//                                 layer_datatype,
//                             )?
//                         }
//                     Geometry::Polygon(p) => {
//                         if !p.interiors.is_empty() {
//                             // TODO: Cut the polygon such that the results have no holes.
//                             unimplemented!("Polygons with holes are not supported yet.");
//                         }
//                         write_simple_polygon(
//                             writer,
//                             &mut modal,
//                             &p.exterior,
//                             layer_index,
//                             layer_datatype,
//                         )?;
//                     }
//                     Geometry::Path(p) => {
//                         write_path(writer, &mut modal, &p, layer_index, layer_datatype)?
//                     }
//                     Geometry::Text(text) => {
//                         write_text(writer, &mut modal, &text, layer_index, layer_datatype)?
//                     }
//                     Geometry::Edge(_) => {
//                         // Edge is not written to layout.
//                     }
//                     Geometry::Point(_) => {
//                         // Not written to layout.
//                     }
//                 }
//             }
//         }
//
//         // Loop through all instances.
//         for inst in layout.each_cell_instance(&cell) {
//             // Write PLACEMENT records.
//             let placement_cell = layout.template_cell(&inst);
//
//             let tf = layout.get_transform(&inst);
//
//             if tf.magnification == 1 {
//                 write_unsigned_integer(writer, 17)?; // PLACEMENT record (with simple representation of rotation and magnification).
//
//                 let (x, y) = tf.displacement.into();
//
//                 // Prepare encoding of the angle in the AA bits of the placement info byte.
//                 let aa = match tf.rotation {
//                     Angle::R0 => 0,
//                     Angle::R90 => 1,
//                     Angle::R180 => 2,
//                     Angle::R270 => 3,
//                 };
//
//                 let is_explicit_reference =  // C
//                     // Use an implicit reference if the current placement cell is the same as the last.
//                     modal.placement_cell.as_ref() != Some(&placement_cell);
//                 // For now, always use references by cell name.
//                 let is_cell_ref_present = false; // N
//
//                 let is_flip = tf.mirror; // F
//
//                 let write_x = modal.placement_x != x; // X
//                 let write_y = modal.placement_y != y; // Y
//                 let write_repetition = false; // R
//
//                 let placement_info_byte = bit(0, is_flip)
//                     | (aa << 1)
//                     | bit(3, write_repetition)
//                     | bit(4, write_y)
//                     | bit(5, write_x)
//                     | bit(6, is_cell_ref_present)
//                     | bit(7, is_explicit_reference);
//
//                 write_byte(writer, placement_info_byte as u8)?;
//
//                 if is_explicit_reference {
//                     if is_cell_ref_present {
//                         // Write placement cell reference number.
//                         unimplemented!();
//                     } else {
//                         // Write placement cell name.
//                         // TODO: Return error instead of expect or generate cell name!
//                         let n = layout.cell_name(&placement_cell);
//                         let name: &str = n.borrow();
//                         write_name_string(writer, name.as_bytes())?;
//                     }
//                 }
//
//                 if write_x {
//                     write_signed_integer(writer, x)?
//                 };
//                 if write_y {
//                     write_signed_integer(writer, y)?
//                 };
//                 if write_repetition {
//                     modal.repetition = None;
//                     unimplemented!();
//                 };
//
//                 modal.placement_cell = Some(placement_cell);
//                 modal.placement_x = x;
//                 modal.placement_y = y;
//             } else {
//                 unimplemented!("Magnifications other than 1 are not supported yet.");
//             }
//         }
//     }
//
//     Ok(())
// }