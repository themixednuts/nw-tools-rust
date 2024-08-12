use std::array::TryFromSliceError;

use serde_json::{json, Value};
use uuid::Uuid;

pub const CHAR: Uuid = Uuid::from_u128(0x3AB0037F_AF8D_48CE_BCA0_A170D18B2C03);
pub const SIGNED_CHAR: Uuid = Uuid::from_u128(0xCFD606FE_41B8_4744_B79F_8A6BD97713D8);
pub const AZ_S8: Uuid = Uuid::from_u128(0x58422C0E_1E47_4854_98E6_34098F6FE12D);
pub const SHORT: Uuid = Uuid::from_u128(0xB8A56D56_A10D_4DCE_9F63_405EE243DD3C);
pub const INT: Uuid = Uuid::from_u128(0x72039442_EB38_4D42_A1AD_CB68F7E0EEF6);
pub const LONG: Uuid = Uuid::from_u128(0x8F24B9AD_7C51_46CF_B2F8_277356957325);
pub const AZ_S64: Uuid = Uuid::from_u128(0x70D8A282_A1EA_462D_9D04_51EDE81FAC2F);
pub const UNSIGNED_CHAR: Uuid = Uuid::from_u128(0x72B9409A_7D1A_4831_9CFE_FCB3FADD3426);
pub const UNSIGNED_SHORT: Uuid = Uuid::from_u128(0xECA0B403_C4F8_4B86_95FC_81688D046E40);
pub const UNSIGNED_INT: Uuid = Uuid::from_u128(0x43DA906B_7DEF_4CA8_9790_854106D3F983);
pub const UNSIGNED_LONG: Uuid = Uuid::from_u128(0x5EC2D6F7_6859_400F_9215_C106F5B10E53);
pub const AZ_U64: Uuid = Uuid::from_u128(0xD6597933_47CD_4FC8_B911_63F3E2B0993A);
pub const FLOAT: Uuid = Uuid::from_u128(0xEA2C3E90_AFBE_44D4_A90D_FAAF79BAF93D);
pub const DOUBLE: Uuid = Uuid::from_u128(0x110C4B14_11A8_4E9D_8638_5051013A56AC);
pub const BOOL: Uuid = Uuid::from_u128(0xA0CA880C_AFE4_43CB_926C_59AC48496112);
pub const AZ_UUID: Uuid = Uuid::from_u128(0xE152C105_A133_4D03_BBF8_3D4B2FBA3E2A);
pub const VOID: Uuid = Uuid::from_u128(0xC0F1AFAD_5CB3_450E_B0F5_ADB5D46B0E22);
pub const CRC32: Uuid = Uuid::from_u128(0x9F4E062E_06A0_46D4_85DF_E0DA96467D3A);
pub const PLATFORM_ID: Uuid = Uuid::from_u128(0x0635D08E_DDD2_48DE_A7AE_73CC563C57C3);
pub const AZSTD_MONOSTATE: Uuid = Uuid::from_u128(0xB1E9136B_D77A_4643_BE8E_2ABDA246AE0E);

pub const AZSTD_LESS: Uuid = Uuid::from_u128(0x41B40AFC_68FD_4ED9_9EC7_BA9992802E1B);
pub const AZSTD_LESS_EQUAL: Uuid = Uuid::from_u128(0x91CC0BDC_FC46_4617_A405_D914EF1C1902);
pub const AZSTD_GREATER: Uuid = Uuid::from_u128(0x907F012A_7A4F_4B57_AC23_48DC08D0782E);
pub const AZSTD_GREATER_EQUAL: Uuid = Uuid::from_u128(0xEB00488F_E20F_471A_B862_F1E3C39DDA1D);
pub const AZSTD_EQUAL_TO: Uuid = Uuid::from_u128(0x4377BCED_F78C_4016_80BB_6AFACE6E5137);
pub const AZSTD_HASH: Uuid = Uuid::from_u128(0xEFA74E54_BDFA_47BE_91A7_5A05DA0306D7);
pub const AZSTD_PAIR: Uuid = Uuid::from_u128(0x919645C1_E464_482B_A69B_04AA688B6847);
pub const AZSTD_VECTOR: Uuid = Uuid::from_u128(0xA60E3E61_1FF6_4982_B6B8_9E4350C4C679);
pub const AZSTD_LIST: Uuid = Uuid::from_u128(0xE1E05843_BB02_4F43_B7DC_3ADB28DF42AC);
pub const AZSTD_FORWARD_LIST: Uuid = Uuid::from_u128(0xD7E91EA3_326F_4019_87F0_6F45924B909A);
pub const AZSTD_SET: Uuid = Uuid::from_u128(0x6C51837F_B0C9_40A3_8D52_2143341EDB07);
pub const AZSTD_UNORDERED_SET: Uuid = Uuid::from_u128(0x8D60408E_DA65_4670_99A2_8ABB574625AE);
pub const AZSTD_UNORDERED_MULTISET: Uuid = Uuid::from_u128(0xB5950921_7F70_4806_9C13_8C7DF841BB90);
pub const AZSTD_MAP: Uuid = Uuid::from_u128(0xF8ECF58D_D33E_49DC_BF34_8FA499AC3AE1);
pub const AZSTD_UNORDERED_MAP: Uuid = Uuid::from_u128(0x41171F6F_9E5E_4227_8420_289F1DD5D005);
pub const AZSTD_UNORDERED_MULTIMAP: Uuid = Uuid::from_u128(0x9ED846FA_31C1_4133_B4F4_91DF9750BA96);
pub const AZSTD_SHARED_PTR: Uuid = Uuid::from_u128(0xFE61C84E_149D_43FD_88BA_1C3DB7E548B4);
pub const AZSTD_INTRUSIVE_PTR: Uuid = Uuid::from_u128(0x530F8502_309E_4EE1_9AEF_5C0456B1F502);
pub const AZSTD_OPTIONAL: Uuid = Uuid::from_u128(0xAB8C50C0_23A7_4333_81CD_46F648938B1C);
pub const AZSTD_BASIC_STRING: Uuid = Uuid::from_u128(0xC26397ED_8F60_4DF6_8320_0D0C592DA3CD);
pub const AZSTD_CHAR_TRAITS: Uuid = Uuid::from_u128(0x9B018C0C_022E_4BA4_AE91_2C1E8592DBB2);
pub const AZSTD_BASIC_STRING_VIEW: Uuid = Uuid::from_u128(0xD348D661_6BDE_4C0A_9540_FCEA4522D497);
pub const AZSTD_FIXED_VECTOR: Uuid = Uuid::from_u128(0x74044B6F_E922_4FD7_915D_EFC5D1DC59AE);
pub const AZSTD_FIXED_LIST: Uuid = Uuid::from_u128(0x508B9687_8410_4A73_AE0C_0BA15CF3F773);
pub const AZSTD_FIXED_FORWARD_LIST: Uuid = Uuid::from_u128(0x0D9D2AB2_F0CC_4E30_A209_A33D78717649);
pub const AZSTD_ARRAY: Uuid = Uuid::from_u128(0x911B2EA8_CCB1_4F0C_A535_540AD00173AE);
pub const AZSTD_BITSET: Uuid = Uuid::from_u128(0x6BAE9836_EC49_466A_85F2_F4B1B70839FB);

pub const VARIANT: Uuid = Uuid::from_u128(0x1E8BB1E5_410A_4367_8FAA_D43A4DE14D4B);
pub const AZSTD_FUNCTION: Uuid = Uuid::from_u128(0xC9F9C644_CCC3_4F77_A792_F5B5DBCA746E);

pub const ASSET: Uuid = Uuid::from_u128(0x77A19D40_8731_4D3C_9041_1B43047366A4);

pub const VECTOR2: Uuid = Uuid::from_u128(0x3D80F623_C85C_4741_90D0_E4E66164E6BF);
pub const VECTOR3: Uuid = Uuid::from_u128(0x8379EB7D_01FA_4538_B64B_A6543B4BE73D);
pub const TRANSFORM: Uuid = Uuid::from_u128(0x5D9958E9_9F1E_4985_B532_FFFDE75FEDFD);
pub const QUATERNION: Uuid = Uuid::from_u128(0x73103120_3DD3_4873_BAB3_9713FA2804FB);
pub const COLOR: Uuid = Uuid::from_u128(0x7894072A_9050_4F0F_901B_34B1A0D29417);
pub const MATRIX3X3: Uuid = Uuid::from_u128(0x15A4332F_7C3F_4A58_AC35_50E1CE53FB9C);

pub fn uuid_data_to_serialize(
    id: &Uuid,
    data: &[u8],
    is_json: bool,
) -> Result<Value, TryFromSliceError> {
    let res = match *id {
        CHAR | AZ_S8 | SIGNED_CHAR => Value::Number(i8::from_be_bytes(data.try_into()?).into()),
        SHORT => Value::Number(i16::from_be_bytes(data.try_into()?).into()),
        INT => Value::Number(i32::from_be_bytes(data.try_into()?).into()),
        LONG | AZ_S64 => Value::Number(i64::from_be_bytes(data.try_into()?).into()),

        UNSIGNED_CHAR => Value::Number(u8::from_be_bytes(data.try_into()?).into()),
        UNSIGNED_SHORT => Value::Number(u16::from_be_bytes(data.try_into()?).into()),
        UNSIGNED_INT => Value::Number(u32::from_be_bytes(data.try_into()?).into()),
        UNSIGNED_LONG | AZ_U64 => Value::Number(u64::from_be_bytes(data.try_into()?).into()),

        FLOAT => json!(format!("{:.7}", f32::from_be_bytes(data.try_into()?))),
        DOUBLE => json!(format!("{:.7}", f64::from_be_bytes(data.try_into()?))),

        BOOL => Value::Bool(u8::from_be_bytes(data.try_into()?) != 0),

        AZ_UUID => json!(Uuid::from_bytes(data.try_into()?)
            .braced()
            .encode_upper(&mut Uuid::encode_buffer())),

        ASSET => {
            let mut buf = Uuid::encode_buffer();
            let guid = Uuid::from_bytes(data[0..16].try_into()?)
                .braced()
                .encode_upper(&mut buf);
            let sub_id = u128::from_be_bytes(data[16..32].try_into()?);
            let mut buf = Uuid::encode_buffer();
            let _type = Uuid::from_bytes(data[32..48].try_into()?)
                .braced()
                .encode_upper(&mut buf);
            let size = u64::from_be_bytes(data[48..56].try_into()?);
            let hint = String::from_utf8_lossy(&data[56..]);
            assert_eq!(hint.len(), size as usize);
            if is_json {
                json!({"assetId": json!({ "guid": guid, "subId": sub_id}), "type": _type, "hint": hint})
            } else {
                json!(format!(
                    "id={}:{},type={},hint={{{}}}",
                    guid, sub_id, _type, hint
                ))
            }
        }

        VECTOR2 | VECTOR3 | TRANSFORM | COLOR | MATRIX3X3 => {
            assert!(data.len() % 4 == 0);
            let data = data.chunks_exact(4);
            let data = data.map(|b| {
                let num = f32::from_be_bytes(b.try_into().unwrap());
                format!("{:.7}", num)
            });

            if is_json {
                Value::Array(data.map(|v| json!(v)).collect())
            } else {
                json!(data.collect::<Vec<_>>().join(" "))
            }
        }

        _ => match String::from_utf8(data.into()) {
            Ok(string) => json!(string),
            _ => json!(""),
        },
    };
    Ok(res)
}
