// Some("int") | Some("short") | Some("char") | Some("AZ::s64") => {
//             match self.data.len() {
//                 1 => {
//                     json["value"] = json!(i8::from_be_bytes(
//                         self.data
//                             .as_slice()
//                             .try_into()
//                             .expect("Vec should be 1 bytes")
//                     ));
//                 }
//                 2 => {
//                     json["value"] = json!(i16::from_be_bytes(
//                         self.data
//                             .as_slice()
//                             .try_into()
//                             .expect("Vec should be 2 bytes")
//                     ));
//                 }
//                 4 => {
//                     json["value"] = json!(i32::from_be_bytes(
//                         self.data
//                             .as_slice()
//                             .try_into()
//                             .expect("Vec should be 4 bytes")
//                     ));
//                 }
//                 8 => {
//                     json["value"] = json!(i64::from_be_bytes(
//                         self.data
//                             .as_slice()
//                             .try_into()
//                             .expect("Vec should be 8 bytes")
//                     ));
//                 }
//                 _ => {}
//             }
//         }
