extern crate byteorder;
extern crate alloc;
extern crate arrayvec;
use std::io::Write;
use std::io::Read;
use std::fs::File;
use self::byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::collections::HashMap;
use std::collections::BinaryHeap;
use std::hash::Hash;
extern crate test;
extern crate bit_vec;
use self::bit_vec::BitVec;

use std;

/// This object represents an error in deserializing or serializinga
/// an item.
#[derive(Debug, Fail)]
#[must_use]
pub enum SavefileError {
    #[fail(display = "Incompatible schema detected: {}", message)]
    IncompatibleSchema {
        message: String,
    },
    #[fail(display = "IO Error: {}",io_error)]
    IOError{io_error:std::io::Error},
    #[fail(display = "Invalid utf8 character {}",msg)]
    InvalidUtf8{msg:String},
    #[fail(display = "Out of memory: {}",err)]
    OutOfMemory{err:std::heap::AllocErr},
    #[fail(display = "Memory allocation failed because memory layout could not be specified.")]
    MemoryAllocationLayoutError,
    #[fail(display = "Arrayvec: {}",msg)]    
    ArrayvecCapacityError{msg:String},
}



/// Object to which serialized data is to be written.
/// This is basically just a wrapped `std::io::Write` object
/// and a file protocol version number.
pub struct Serializer<'a> {
    writer: &'a mut Write,
    pub version: u32,
}

/// Object from which bytes to be deserialized are read.
/// This is basically just a wrapped `std::io::Read` object,
/// the version number of the file being read, and the
/// current version number of the data structures in memory.
pub struct Deserializer<'a> {
    reader: &'a mut Read,
    pub file_version: u32,
    pub memory_version: u32,
}




/// This is a marker trait for types which have an in-memory layout that is packed
/// and therefore identical to the layout that savefile will use on disk.
/// This means that types for which this trait is implemented can be serialized
/// very quickly by just writing their raw bits to disc.
///
/// Rules to implement this trait:
///
/// * The type must be copy
/// * The type must not contain any padding
/// * The type must have a strictly deterministic memory layout (no field order randomization). This typically means repr(C)
/// * All the constituent types of the type must also implement `ReprC` (correctly).
pub unsafe trait ReprC: Copy {
    /// This method returns true if the optimization is allowed
    /// for the protocol version given as an argument.
    /// This may return true if and only if the given protocol version
    /// has a serialized format identical to the given protocol version.
    fn repr_c_optimization_safe(version: u32) -> bool;
}

impl From<std::io::Error> for SavefileError {
    fn from(s: std::io::Error) -> SavefileError {
        SavefileError::IOError{io_error:s}
    }
}

impl From<std::heap::AllocErr> for SavefileError {
    fn from(s: std::heap::AllocErr) -> SavefileError {
        SavefileError::OutOfMemory{err:s}
    }
}

impl From<std::string::FromUtf8Error> for SavefileError {
    fn from(s: std::string::FromUtf8Error) -> SavefileError {
        SavefileError::InvalidUtf8{msg:s.to_string()}
    }
}

impl<T> From<arrayvec::CapacityError<T> > for SavefileError {
    fn from(s: arrayvec::CapacityError<T>) -> SavefileError {
        SavefileError::ArrayvecCapacityError{msg:s.to_string()}
    }
}



#[allow(unit_arg)]
impl<'a> Serializer<'a> {
    pub fn write_bool(&mut self, v: bool)  -> Result<(),SavefileError> {
        Ok(self.writer.write_u8( if v {1} else {0})?)
    }
    pub fn write_u8(&mut self, v: u8)  -> Result<(),SavefileError> {
        Ok(self.writer.write_all(&[v])?)
    }
    pub fn write_i8(&mut self, v: i8) -> Result<(),SavefileError> {
        Ok(self.writer.write_i8(v)?)
    }

    pub fn write_u16(&mut self, v: u16) -> Result<(),SavefileError> {
        Ok(self.writer.write_u16::<LittleEndian>(v)?)
    }
    pub fn write_i16(&mut self, v: i16) -> Result<(),SavefileError> {
        Ok(self.writer.write_i16::<LittleEndian>(v)?)
    }

    pub fn write_u32(&mut self, v: u32) -> Result<(),SavefileError> {
        Ok(self.writer.write_u32::<LittleEndian>(v)?)
    }
    pub fn write_i32(&mut self, v: i32) -> Result<(),SavefileError> {
        Ok(self.writer.write_i32::<LittleEndian>(v)?)
    }

    pub fn write_f32(&mut self, v: f32) -> Result<(),SavefileError> {
        Ok(self.writer.write_f32::<LittleEndian>(v)?)
    }
    pub fn write_f64(&mut self, v: f64) -> Result<(),SavefileError> {
        Ok(self.writer.write_f64::<LittleEndian>(v)?)
    }

    pub fn write_u64(&mut self, v: u64) -> Result<(),SavefileError> {
        Ok(self.writer.write_u64::<LittleEndian>(v)?)
    }
    pub fn write_i64(&mut self, v: i64) -> Result<(),SavefileError> {
        Ok(self.writer.write_i64::<LittleEndian>(v)?)
    }

    pub fn write_usize(&mut self, v: usize) -> Result<(),SavefileError> {
        Ok(self.writer.write_u64::<LittleEndian>(v as u64)?)
    }
    pub fn write_isize(&mut self, v: isize) -> Result<(),SavefileError> {
        Ok(self.writer.write_i64::<LittleEndian>(v as i64)?)
    }
    pub fn write_buf(&mut self, v: &[u8]) -> Result<(),SavefileError> {
        Ok(self.writer.write_all(v)?)
    }
    pub fn write_string(&mut self, v: &str) -> Result<(),SavefileError> {
        let asb = v.as_bytes();
        self.write_usize(asb.len())?;
        Ok(self.writer.write_all(asb)?)
    }
    pub fn write_bytes(&mut self, v: &[u8]) -> Result<(),SavefileError> {
        Ok(self.writer.write_all(v)?)
    }

    /// Creata a new serializer.
    /// Don't use this function directly, use the [savefile::save] function instead.
    pub fn save<T:WithSchema + Serialize>(writer: &mut Write, version: u32, data: &T) -> Result<(),SavefileError> {
        Ok(Self::save_impl(writer,version,data,true)?)
    }
    /// Creata a new serializer.
    /// Don't use this function directly, use the [savefile::save_noschema] function instead.
    pub fn save_noschema<T:WithSchema + Serialize>(writer: &mut Write, version: u32, data: &T) -> Result<(),SavefileError> {
        Ok(Self::save_impl(writer,version,data,false)?)
    }
    fn save_impl<T:WithSchema + Serialize>(writer: &mut Write, version: u32, data: &T, with_schema: bool) -> Result<(),SavefileError> {
        writer.write_u32::<LittleEndian>(version).unwrap();

        if with_schema
        {
            let schema = T::schema(version);
            let mut schema_serializer=Serializer::new_raw(writer);
            schema.serialize(&mut schema_serializer)?;            
        }

        let mut serializer=Serializer { writer, version };
        Ok(data.serialize(&mut serializer)?)
    }

    /// Create a Serializer.
    /// Don't use this method directly, use the [savefile::save] function
    /// instead.
    pub fn new_raw(writer: &mut Write) -> Serializer {
        Serializer { writer, version:0 }
    }
}

impl<'a> Deserializer<'a> {
    pub fn read_bool(&mut self) -> Result<bool,SavefileError> {
        Ok(self.reader.read_u8()? == 1)
    }
    pub fn read_u8(&mut self) -> Result<u8,SavefileError> {
        let mut buf = [0u8];
        self.reader.read_exact(&mut buf)?;
        Ok(buf[0])
    }
    pub fn read_u16(&mut self) -> Result<u16,SavefileError> {
        Ok(self.reader.read_u16::<LittleEndian>()?)
    }
    pub fn read_u32(&mut self) -> Result<u32,SavefileError> {
        Ok(self.reader.read_u32::<LittleEndian>()?)
    }
    pub fn read_u64(&mut self) -> Result<u64,SavefileError> {
        Ok(self.reader.read_u64::<LittleEndian>()?)
    }

    pub fn read_i8(&mut self) -> Result<i8,SavefileError> {
        Ok(self.reader.read_i8()?)
    }
    pub fn read_i16(&mut self) -> Result<i16,SavefileError> {
        Ok(self.reader.read_i16::<LittleEndian>()?)
    }
    pub fn read_i32(&mut self) -> Result<i32,SavefileError> {
        Ok(self.reader.read_i32::<LittleEndian>()?)
    }
    pub fn read_i64(&mut self) -> Result<i64,SavefileError> {
        Ok(self.reader.read_i64::<LittleEndian>()?)
    }
    pub fn read_f32(&mut self) -> Result<f32,SavefileError> {
        Ok(self.reader.read_f32::<LittleEndian>()?)
    }
    pub fn read_f64(&mut self) -> Result<f64,SavefileError> {
        Ok(self.reader.read_f64::<LittleEndian>()?)
    }
    pub fn read_isize(&mut self) -> Result<isize,SavefileError> {
        Ok(self.reader.read_i64::<LittleEndian>()? as isize)
    }
    pub fn read_usize(&mut self) -> Result<usize,SavefileError> {
        Ok(self.reader.read_u64::<LittleEndian>()? as usize)
    }
    pub fn read_string(&mut self) -> Result<String,SavefileError> {
        let l = self.read_usize()?;
        let mut v = Vec::with_capacity(l);
        v.resize(l, 0); //TODO: Optimize this
        self.reader.read_exact(&mut v)?;
        Ok(String::from_utf8(v)?)
    }
    pub fn read_bytes(&mut self, len:usize) -> Result<Vec<u8>,SavefileError> {
        let mut v = Vec::with_capacity(len);
        v.resize(len, 0); //TODO: Optimize this
        self.reader.read_exact(&mut v)?;
        Ok(v)        
    }

    /// Deserialize an object of type T from the given reader.
    /// Don't use this method directly, use the [savefile::load] function
    /// instead.
    pub fn load<T:WithSchema+Deserialize>(reader: &mut Read, version: u32) -> Result<T,SavefileError> {
        Deserializer::load_impl::<T>(reader,version,true)
    }
    /// Deserialize an object of type T from the given reader.
    /// Don't use this method directly, use the [savefile::load_noschema] function
    /// instead.
    pub fn load_noschema<T:WithSchema+Deserialize>(reader: &mut Read, version: u32) -> Result<T,SavefileError> {
        Deserializer::load_impl::<T>(reader,version,false)
    }
    fn load_impl<T:WithSchema+Deserialize>(reader: &mut Read, version: u32, fetch_schema: bool) -> Result<T,SavefileError> {
        let file_ver = reader.read_u32::<LittleEndian>()?;
        if file_ver > version {
            panic!(
                "File has later version ({}) than structs in memory ({}).",
                file_ver, version
            );
        }

        if fetch_schema
        {
            let mut schema_deserializer = Deserializer::new_raw(reader);
            let memory_schema = T::schema(file_ver);
            let file_schema = Schema::deserialize(&mut schema_deserializer)?;
            
            if let Some(err) = diff_schema(&file_schema, &memory_schema,".".to_string()) {
                return Err(SavefileError::IncompatibleSchema{
                    message:format!("Saved schema differs from in-memory schema for version {}. Error: {}",file_ver,
                    err)});
            }
        }
        let mut deserializer=Deserializer {
            reader,
            file_version: file_ver,
            memory_version: version,
        };
        Ok(T::deserialize(&mut deserializer)?)
    }

    /// Create a Deserializer.
    /// Don't use this method directly, use the [savefile::load] function
    /// instead.
    pub fn new_raw(reader: &mut Read) -> Deserializer {
        Deserializer {
            reader,
            file_version: 0,
            memory_version: 0,
        }
    }
}


/// Deserialize an instance of type T from the given `reader` .
/// The current type of T in memory must be equal to `version`.
/// The deserializer will use the actual protocol version in the
/// file to do the deserialization.
pub fn load<T:WithSchema+Deserialize>(reader: &mut Read, version: u32) -> Result<T,SavefileError> {
    Deserializer::load::<T>(reader,version)
}

/// Write the given `data` to the `writer`. 
/// The current version of data must be `version`.
pub fn save<T:WithSchema+Serialize>(writer: &mut Write, version: u32, data: &T) -> Result<(),SavefileError> {
    Serializer::save::<T>(writer,version,data)
}

/// Like [savefile::load] , but used to open files saved without schema,
/// by one of the _noschema versions of the save functions.
pub fn load_noschema<T:WithSchema+Deserialize>(reader: &mut Read, version: u32) -> Result<T,SavefileError> {
    Deserializer::load_noschema::<T>(reader,version)
}

/// Write the given `data` to the `writer`. 
/// The current version of data must be `version`.
/// Do this write without writing any schema to disk.
/// As long as all the serializers and deserializers
/// are correctly written, the schema is not necessary.
/// Omitting the schema saves some space in the saved file,
/// but means that any mistake in implementation of the 
/// Serialize or Deserialize traits will cause hard-to-troubleshoot
/// data corruption instead of a nice error message.
pub fn save_noschema<T:WithSchema+Serialize>(writer: &mut Write, version: u32, data: &T) -> Result<(),SavefileError> {
    Serializer::save_noschema::<T>(writer,version,data)
}

/// Like [savefile::load] , except it deserializes from the given file in the filesystem.
/// This is a pure convenience function.
pub fn load_file<T:WithSchema+Deserialize>(filepath:&str, version: u32) -> Result<T,SavefileError> {
    let mut f = File::open(filepath)?;
    Deserializer::load::<T>(&mut f, version)
}

/// Like [savefile::save] , except it opens a file on the filesystem and writes
/// the data to it. This is a pure convenience function.
pub fn save_file<T:WithSchema+Serialize>(filepath:&str, version: u32, data: &T) -> Result<(),SavefileError> {
    let mut f = File::create(filepath)?;
    Serializer::save::<T>(&mut f,version,data)
}

/// Like [savefile::load_noschema] , except it deserializes from the given file in the filesystem.
/// This is a pure convenience function.
pub fn load_file_noschema<T:WithSchema+Deserialize>(filepath:&str, version: u32) -> Result<T,SavefileError> {
    let mut f = File::open(filepath)?;
    Deserializer::load_noschema::<T>(&mut f,version)
}

/// Like [savefile::save_noschema] , except it opens a file on the filesystem and writes
/// the data to it. This is a pure convenience function.
pub fn save_file_noschema<T:WithSchema+Serialize>(filepath:&str, version: u32, data: &T) -> Result<(),SavefileError> {
    let mut f = File::create(filepath)?;
    Serializer::save_noschema::<T>(&mut f,version,data)
}






/// This trait must be implemented by all data structures you wish to be able to save.
/// It must encode the schema for the datastructure when saved using the given version number.
/// When files are saved, the schema is encoded into the file.
/// when loading, the schema is inspected to make sure that the load will safely succeed.
/// This is only for increased safety, the file format does not in fact use the schema for any other
/// purpose, the design is schema-less at the core, the schema is just an added layer of safety (which
/// can be disabled).
pub trait WithSchema {
    /// Returns a representation of the schema used by this Serialize implementation for the given version.
    fn schema(version:u32) -> Schema;    
}

/// This trait must be implemented for all data structures you wish to be
/// able to serialize. To actually serialize data: create a [Serializer],
/// then call serialize on your data to save, giving the Serializer
/// as an argument.
///
/// The most convenient way to implement this is to use
/// `#[macro_use]
/// extern crate savefile-derive;`
///
/// and the use #[derive(Serialize)]
pub trait Serialize : WithSchema {
    /// Serialize self into the given serializer.
    fn serialize(&self, serializer: &mut Serializer) -> Result<(),SavefileError>; //TODO: Do error handling

}

/// This trait must be implemented for all data structures you wish to
/// be able to deserialize.
///
/// The most convenient way to implement this is to use
/// `#[macro_use]
/// extern crate savefile-derive;`
///
/// and the use #[derive(Deserialize)]
pub trait Deserialize : WithSchema + Sized {
    /// Deserialize and return an instance of Self from the given deserializer.
    fn deserialize(deserializer: &mut Deserializer) -> Result<Self,SavefileError>;  //TODO: Do error handling
}



/// A field is serialized according to its value.
/// The name is just for diagnostics.
#[derive(Debug,PartialEq)]
pub struct Field {
    pub name : String,
    pub value : Box<Schema>
}

/// A struct is by serializing its fields one by one,
/// without any padding. 
/// The dbg_name is just for diagnostics.
#[derive(Debug,PartialEq)]
pub struct SchemaStruct {
    pub dbg_name : String,
    pub fields : Vec<Field>
}
fn maybe_add(a:Option<usize>,b:Option<usize>) -> Option<usize> {
    if let Some(a) = a {
        if let Some(b) = b {
            return Some(a+b);
        }
    }   
    None 
}
impl SchemaStruct {
    fn serialized_size(&self) -> Option<usize> {
        self.fields.iter().fold(Some(0usize),|prev,x| {
            maybe_add(prev,x.value.serialized_size())
        })
    }
}


/// An enum variant is serialized as its fields, one by one,
/// without any padding.
#[derive(Debug,PartialEq)]
pub struct Variant {
    pub name : String,
    pub discriminator : u8,
    pub fields : Vec<Field>
}
impl Variant {
    fn serialized_size(&self) -> Option<usize> {
        self.fields.iter().fold(Some(0usize),|prev,x| {
            maybe_add(prev,x.value.serialized_size())
        })
    }
}

/// An enum is serialized as its u8 variant discriminator
/// followed by all the field for that variant.
/// The name of each variant, as well as its order in 
/// the enum (the discriminator), is significant.
#[derive(Debug, PartialEq)]
pub struct SchemaEnum {
    pub dbg_name : String,
    pub variants : Vec<Variant>
}


fn maybe_max(a:Option<usize>,b:Option<usize>) -> Option<usize> {
    if let Some(a) = a {
        if let Some(b) = b {
            return Some(a.max(b));
        }
    }   
    None 
}
impl SchemaEnum {
    fn serialized_size(&self) -> Option<usize> {
        self.variants.iter().fold(Some(0usize),|prev,x| {
            maybe_max(prev,x.serialized_size())
        })
    }
}


/// A primitive is serialized as the little endian
/// representation of its type, except for string,
/// which is serialized as an usize length followed
/// by the string in utf8.
#[allow(non_camel_case_types)]
#[derive(Copy,Clone,Debug,PartialEq)]
pub enum SchemaPrimitive {
    schema_i8,
    schema_u8,
    schema_i16,
    schema_u16,
    schema_i32,
    schema_u32,
    schema_i64,
    schema_u64,
    schema_string,
    schema_f32,
    schema_f64,
    schema_bool,
}
impl SchemaPrimitive {
    fn name(&self) -> &'static str {
        match *self {
            SchemaPrimitive::schema_i8 => "i8",
            SchemaPrimitive::schema_u8 => "u8",
            SchemaPrimitive::schema_i16 => "i16",
            SchemaPrimitive::schema_u16 => "u16",
            SchemaPrimitive::schema_i32 => "i32",
            SchemaPrimitive::schema_u32 => "u32",
            SchemaPrimitive::schema_i64 => "i64",
            SchemaPrimitive::schema_u64 => "u64",
            SchemaPrimitive::schema_string => "String",
            SchemaPrimitive::schema_f32 => "f32",
            SchemaPrimitive::schema_f64 => "f64",
            SchemaPrimitive::schema_bool => "bool",
        }
    }
}

impl SchemaPrimitive {
    fn serialized_size(&self) -> Option<usize> {

        match *self {
            SchemaPrimitive::schema_i8 | SchemaPrimitive::schema_u8 => Some(1),
            SchemaPrimitive::schema_i16 | SchemaPrimitive::schema_u16 => Some(2),
            SchemaPrimitive::schema_i32 | SchemaPrimitive::schema_u32 => Some(4),
            SchemaPrimitive::schema_i64 | SchemaPrimitive::schema_u64 => Some(8),
            SchemaPrimitive::schema_string => None,       
            SchemaPrimitive::schema_f32 => Some(4),
            SchemaPrimitive::schema_f64 => Some(8),
            SchemaPrimitive::schema_bool => Some(1),
        }
    }
}

fn diff_primitive(a:SchemaPrimitive,b:SchemaPrimitive, path:&str) -> Option<String> {
    if a!=b {

        return Some(format!("At location [{}]: Application protocol has datatype {}, but disk format has {}",
            path,a.name(),b.name()));
    }
    None
}


/// The schema represents the save file format
/// of your data. 
#[derive(Debug,PartialEq)]
pub enum Schema {
    Struct(SchemaStruct),
    Enum(SchemaEnum),
    Primitive(SchemaPrimitive),
    Vector(Box<Schema>),
    SchemaOption(Box<Schema>),
    Undefined,
    ZeroSize,
}

impl Schema {
    pub fn new_tuple1<T1:WithSchema>(version:u32) -> Schema {
        Schema::Struct(
            SchemaStruct {
                dbg_name: "1-Tuple".to_string(),
                fields:vec![
                    Field {name:"0".to_string(), value: Box::new(T1::schema(version))},
                ]
            })
    }

    pub fn new_tuple2<T1:WithSchema,T2:WithSchema>(version:u32) -> Schema {
        Schema::Struct(
            SchemaStruct {
                dbg_name: "2-Tuple".to_string(),
                fields:vec![
                    Field {name:"0".to_string(), value: Box::new(T1::schema(version))},
                    Field {name:"1".to_string(), value: Box::new(T2::schema(version))}
                ]
            })
    }
    pub fn new_tuple3<T1:WithSchema,T2:WithSchema,T3:WithSchema>(version:u32) -> Schema {
        Schema::Struct(
            SchemaStruct {
                dbg_name: "3-Tuple".to_string(),
                fields:vec![
                    Field {name:"0".to_string(), value: Box::new(T1::schema(version))},
                    Field {name:"1".to_string(), value: Box::new(T2::schema(version))},
                    Field {name:"2".to_string(), value: Box::new(T3::schema(version))}
                ]
            })
    }
    pub fn serialized_size(&self) -> Option<usize> {
        match *self {
            Schema::Struct(ref schema_struct) => {
                schema_struct.serialized_size()
            }
            Schema::Enum(ref schema_enum) => {
                schema_enum.serialized_size()
            }
            Schema::Primitive(ref schema_primitive) => {
                schema_primitive.serialized_size()
            }
            Schema::Vector(ref _vector) => {
                None
            }
            Schema::SchemaOption(ref _content) => {
                None
            }
            Schema::Undefined => {
                None
            }
            Schema::ZeroSize => {
                Some(0)
            }
        }
    }
}

fn diff_vector(a:&Schema,b:&Schema,path:String) -> Option<String> {
    diff_schema(a,b,
        path + "/*")
}

fn diff_option(a:&Schema,b:&Schema,path:String) -> Option<String> {
    diff_schema(a,b,
        path + "/?")
}

fn diff_enum(a:&SchemaEnum,b:&SchemaEnum, path:String)  -> Option<String> {

    let path = (path + &b.dbg_name).to_string();
    if a.variants.len()!=b.variants.len() {
        return Some(format!("At location [{}]: In memory enum has {} variants, but disk format has {} variants.",
            path,a.variants.len(),b.variants.len()));
    }
    for i in 0..a.variants.len() {
        if a.variants[i].name!=b.variants[i].name {
            return Some(format!("At location [{}]: Enum variant #{} in memory is called {}, but in disk format it is called {}",
                &path,i, a.variants[i].name,b.variants[i].name));
        }
        if a.variants[i].discriminator!=b.variants[i].discriminator {
            return Some(format!("At location [{}]: Enum variant #{} in memory has discriminator {}, but in disk format it has {}",
                &path,i,a.variants[i].discriminator,b.variants[i].discriminator));
        }
        let r=diff_fields(&a.variants[i].fields,&b.variants[i].fields,&(path.to_string()+"/"+&b.variants[i].name).to_string(),"enum",
            "","");
        if let Some(err)=r {
            return Some(err);
        }
    }
    None
}
fn diff_struct(a:&SchemaStruct,b:&SchemaStruct,path:String) -> Option<String> {
    diff_fields(&a.fields,&b.fields,&(path+"/"+&b.dbg_name).to_string(),"struct", 
        &(" (struct ".to_string()+&a.dbg_name+")"), &(" (struct ".to_string()+&b.dbg_name+")"))
}
fn diff_fields(a:&[Field],b:&[Field],path:&str, structuretype:&str,
    extra_a:&str,extra_b:&str) -> Option<String> {
    if a.len()!=b.len() {
        return Some(format!("At location [{}]: In memory {}{} has {} fields, disk format{} has {} fields.",
            path,structuretype,extra_a,a.len(),extra_b,b.len()));
    }
    for i in 0..a.len() {
        /*
        if a[i].name!=b[i].name {
            return Some(format!("At location [{}]: Field #{} in memory{} is called {}, but in disk format{} it is called {}",
                &path,i,extra_a,a[i].name,extra_b,b[i].name));
        }*/
        let r=diff_schema(&a[i].value,&b[i].value,(path.to_string()+"/"+&b[i].name).to_string());
        if let Some(err)=r {
            return Some(err);
        }
    }
    None
}

/// Return a (kind of) human-readable description of the difference
/// between the two schemas. The schema 'a' is assumed to be the current
/// schema (used in memory).
/// Returns None if both schemas are equivalent
fn diff_schema(a:&Schema, b: &Schema, path:String) -> Option<String> {
    let (atype,btype)=match *a {
        Schema::Struct(ref xa) => {
            match *b {
                Schema::Struct(ref xb) => {
                    return diff_struct(xa,xb,path)
                },
                Schema::Enum(_) => ("struct","enum"),
                Schema::Primitive(_) => ("struct","primitive"),
                Schema::Vector(_) => ("struct","vector"),
                Schema::SchemaOption(_) => ("struct","option"),
                Schema::Undefined => ("struct","undefined"),
                Schema::ZeroSize => ("struct","zerosize"),
            }
        }
        Schema::Enum(ref xa) => {
            match *b {
                Schema::Enum(ref xb) => {
                    return diff_enum(xa,xb,path)
                },
                Schema::Struct(_) => ("enum","struct"),
                Schema::Primitive(_) => ("enum","primitive"),
                Schema::Vector(_) => ("enum","vector"),
                Schema::SchemaOption(_) => ("enum","option"),
                Schema::Undefined => ("enum","undefined"),
                Schema::ZeroSize => ("enum","zerosize"),
            }
        }
        Schema::Primitive(ref xa) => {
            match *b {
                Schema::Primitive(ref xb) => {
                    return diff_primitive(*xa,*xb,&path);
                },
                Schema::Struct(_) => ("primitive","struct"),
                Schema::Enum(_) => ("primitive","enum"),
                Schema::Vector(_) => ("primitive","vector"),
                Schema::SchemaOption(_) => ("primitive","option"),
                Schema::Undefined => ("primitive","undefined"),
                Schema::ZeroSize => ("primitive","zerosize"),
            }
        }
        Schema::SchemaOption(ref xa) => {
            match *b {
                Schema::SchemaOption(ref xb) => {
                    return diff_option(xa,xb,path);
                },
                Schema::Struct(_) => ("option","struct"),
                Schema::Enum(_) => ("option","enum"),
                Schema::Primitive(_) => ("option","primitive"),
                Schema::Vector(_) => ("option","vector"),
                Schema::Undefined => ("option","undefined"),
                Schema::ZeroSize => ("option","zerosize"),
            }            
        }
        Schema::Vector(ref xa) => {
            match *b {
                Schema::Vector(ref xb) => {
                    return diff_vector(xa,xb,path);
                },
                Schema::Struct(_) => ("vector","struct"),
                Schema::Enum(_) => ("vector","enum"),
                Schema::Primitive(_) => ("vector","primitive"),
                Schema::SchemaOption(_) => ("vector","option"),
                Schema::Undefined => ("vector","undefined"),
                Schema::ZeroSize => ("vector","zerosize"),
            }
        }
        Schema::Undefined => {
            return Some(format!("At location [{}]: Undefined schema encountered.",path));
        }
        Schema::ZeroSize => {
            match *b {
                Schema::ZeroSize => {
                    return None;
                },
                Schema::Vector(_) => ("zerosize","vector"),
                Schema::Struct(_) => ("zerosize","struct"),
                Schema::Enum(_) => ("zerosize","enum"),
                Schema::SchemaOption(_) => ("zerosize","option"),
                Schema::Primitive(_) => ("zerosize","primitive"),
                Schema::Undefined => ("zerosize","undefined"),
            }
        }
    };
    Some(format!("At location [{}]: In memory schema: {}, file schema: {}",
        path,atype,btype))
    
}


impl WithSchema for Field {
    fn schema(_version:u32) -> Schema {
        Schema::Undefined
    }    
}
impl Serialize for Field {
    fn serialize(&self, serializer: &mut Serializer) -> Result<(),SavefileError>{
        serializer.write_string(&self.name)?;
        self.value.serialize(serializer)
    }
}
impl Deserialize for Field {
    fn deserialize(deserializer: &mut Deserializer) -> Result<Self,SavefileError> {
        Ok(Field {
            name : deserializer.read_string()?,
            value : Box::new(Schema::deserialize(deserializer)?)
        })
    }
}
impl WithSchema for Variant {
    fn schema(_version:u32) -> Schema {
        Schema::Undefined
    }    
}
impl Serialize for Variant {
    fn serialize(&self, serializer: &mut Serializer) -> Result<(),SavefileError> {
        serializer.write_string(&self.name)?;
        serializer.write_u8(self.discriminator)?;
        serializer.write_usize(self.fields.len())?;
        for field in &self.fields  {
            field.serialize(serializer)?;
        }
        Ok(())
    }    
}
impl Deserialize for Variant {
    fn deserialize(deserializer: &mut Deserializer) -> Result<Self,SavefileError> {
        Ok(Variant {
            name: deserializer.read_string()?,
            discriminator: deserializer.read_u8()?,
            fields : {
                let l = deserializer.read_usize()?;
                let mut ret=Vec::new();
                for _ in 0..l {
                    ret.push(
                        Field {
                            name: deserializer.read_string()?,
                            value: Box::new(Schema::deserialize(deserializer)?)
                        });
                }
                ret
            }
        })
    }
}

impl WithSchema for SchemaStruct {
    fn schema(_version:u32) -> Schema {
        Schema::Undefined
    }
}
impl Serialize for SchemaStruct {
    fn serialize(&self, serializer: &mut Serializer) -> Result<(),SavefileError> {
        serializer.write_string(&self.dbg_name)?;
        serializer.write_usize(self.fields.len())?;
        for field in &self.fields {            
            field.serialize(serializer)?;
        }
        Ok(())
    }
}
impl Deserialize for SchemaStruct {
    fn deserialize(deserializer: &mut Deserializer) -> Result<Self,SavefileError> {
        let dbg_name = deserializer.read_string()?;
        let l=deserializer.read_usize()?;
        Ok(SchemaStruct {
            dbg_name,
            fields: {
                let mut ret=Vec::new();
                for _ in 0..l {
                    ret.push(Field::deserialize(deserializer)?)
                }
                ret
            }        
        })
    }
}

impl WithSchema for SchemaPrimitive {
    fn schema(_version:u32) -> Schema {
        Schema::Undefined
    }    
}
impl Serialize for SchemaPrimitive {
    fn serialize(&self, serializer: &mut Serializer) -> Result<(),SavefileError> {
        let discr=match *self {
            SchemaPrimitive::schema_i8 => 1,
            SchemaPrimitive::schema_u8 => 2,
            SchemaPrimitive::schema_i16 => 3,
            SchemaPrimitive::schema_u16 => 4,
            SchemaPrimitive::schema_i32 => 5,
            SchemaPrimitive::schema_u32 => 6,
            SchemaPrimitive::schema_i64 => 7,
            SchemaPrimitive::schema_u64 => 8,
            SchemaPrimitive::schema_string => 9,
            SchemaPrimitive::schema_f32 => 10,
            SchemaPrimitive::schema_f64 => 11,
            SchemaPrimitive::schema_bool => 12,
        };
        serializer.write_u8(discr)
    }
}
impl Deserialize for SchemaPrimitive {
    fn deserialize(deserializer: &mut Deserializer) -> Result<Self,SavefileError> {
        let var=match deserializer.read_u8()? {
            1 => SchemaPrimitive::schema_i8,
            2 => SchemaPrimitive::schema_u8,
            3 => SchemaPrimitive::schema_i16,
            4 => SchemaPrimitive::schema_u16,
            5 => SchemaPrimitive::schema_i32,
            6 => SchemaPrimitive::schema_u32,
            7 => SchemaPrimitive::schema_i64,
            8 => SchemaPrimitive::schema_u64,
            9 => SchemaPrimitive::schema_string,
            10 => SchemaPrimitive::schema_f32,
            11 => SchemaPrimitive::schema_f64,
            12 => SchemaPrimitive::schema_bool,
            c => panic!("Corrupt schema, primitive type #{} encountered",c),
        };
        Ok(var)
    }
}

impl WithSchema for SchemaEnum {
    fn schema(_version:u32) -> Schema {
        Schema::Undefined
    }    
}

impl Serialize for SchemaEnum {
    fn serialize(&self, serializer: &mut Serializer) -> Result<(),SavefileError> {
        serializer.write_string(&self.dbg_name)?;
        serializer.write_usize(self.variants.len())?;
        for var in &self.variants {            
            var.serialize(serializer)?;
        }
        Ok(())
    }
}
impl Deserialize for SchemaEnum {
    fn deserialize(deserializer:&mut Deserializer) -> Result<Self,SavefileError> {
        let dbg_name = deserializer.read_string()?;
        let l = deserializer.read_usize()?;
        let mut ret=Vec::new();
        for _ in 0..l {
            ret.push(Variant::deserialize(deserializer)?);
        }
        Ok(SchemaEnum {
            dbg_name,
            variants: ret
        })
    }
}

impl WithSchema for Schema {
    fn schema(_version:u32) -> Schema {
        Schema::Undefined
    }    
}
impl Serialize for Schema {
    fn serialize(&self, serializer: &mut Serializer) -> Result<(),SavefileError> {
        match *self {
            Schema::Struct(ref schema_struct) => {
                serializer.write_u8(1)?;
                schema_struct.serialize(serializer)
            },
            Schema::Enum(ref schema_enum) => {
                serializer.write_u8(2)?;
                schema_enum.serialize(serializer)
            },
            Schema::Primitive(ref schema_prim) => {
                serializer.write_u8(3)?;
                schema_prim.serialize(serializer)
            },
            Schema::Vector(ref schema_vector) => {
                serializer.write_u8(4)?;
                schema_vector.serialize(serializer)
            },
            Schema::Undefined => {
                serializer.write_u8(5)
            },
            Schema::ZeroSize => {
                serializer.write_u8(6)
            },
            Schema::SchemaOption(ref content) => {
                serializer.write_u8(7)?;
                content.serialize(serializer)
            },
        }
    }    
}


impl Deserialize for Schema {
    fn deserialize(deserializer: &mut Deserializer) -> Result<Self,SavefileError> {
        let schema=match deserializer.read_u8()? {
            1 => Schema::Struct(SchemaStruct::deserialize(deserializer)?),
            2 => Schema::Enum(SchemaEnum::deserialize(deserializer)?),
            3 => Schema::Primitive(SchemaPrimitive::deserialize(deserializer)?),
            4 => Schema::Vector(Box::new(Schema::deserialize(deserializer)?)),
            5 => Schema::Undefined,
            6 => Schema::ZeroSize,
            7 => Schema::SchemaOption(Box::new(Schema::deserialize(deserializer)?)),
            c => panic!("Corrupt schema, schema variant had value {}", c),
        };

        Ok(schema)

    }
}

impl WithSchema for String {
    fn schema(_version:u32) -> Schema {
        Schema::Primitive(SchemaPrimitive::schema_string)
    }    
}

impl Serialize for String {
    fn serialize(&self, serializer: &mut Serializer) -> Result<(),SavefileError> {
        serializer.write_string(self)
    }
}

impl Deserialize for String {
    fn deserialize(deserializer: &mut Deserializer) -> Result<String,SavefileError> {
        deserializer.read_string()
    }
}


impl<K: WithSchema + Eq + Hash, V: WithSchema, S: ::std::hash::BuildHasher> WithSchema
    for HashMap<K, V, S> {
    fn schema(version:u32) -> Schema {
        Schema::Vector(Box::new(
            Schema::Struct(SchemaStruct{
                dbg_name: "KeyValuePair".to_string(),
                fields: vec![
                    Field {
                        name: "key".to_string(),
                        value: Box::new(K::schema(version)),
                    },
                    Field {
                        name: "value".to_string(),
                        value: Box::new(V::schema(version)),
                    },
                ]
            })))
    }        
}


impl<K: Serialize + Eq + Hash, V: Serialize, S: ::std::hash::BuildHasher> Serialize
    for HashMap<K, V, S>
{
    fn serialize(&self, serializer: &mut Serializer) -> Result<(),SavefileError> {
        serializer.write_usize(self.len())?;
        for (k, v) in self.iter() {
            k.serialize(serializer)?;
            v.serialize(serializer)?;
        }
        Ok(())
    }
}


impl<K: Deserialize + Eq + Hash, V: Deserialize> Deserialize for HashMap<K, V> {
    fn deserialize(deserializer: &mut Deserializer) -> Result<Self,SavefileError> {
        let l = deserializer.read_usize()?;
        let mut ret = HashMap::with_capacity(l);
        for _ in 0..l {
            ret.insert(K::deserialize(deserializer)?, V::deserialize(deserializer)?);
        }
        Ok(ret)
    }
}

#[derive(Debug, PartialEq)]
pub struct Removed<T> {
    phantom: std::marker::PhantomData<T>,
}

#[allow(new_without_default_derive)] //You should never need to instantiate Removed yourself anyway.
impl<T> Removed<T> {
    pub fn new() -> Removed<T> {
        Removed {
            phantom: std::marker::PhantomData,
        }
    }
}
impl<T:WithSchema> WithSchema for Removed<T> {
    fn schema(version:u32) -> Schema {
        <T>::schema(version)
    }    
}


impl<T:WithSchema> Serialize for Removed<T> {
    fn serialize(&self, _serializer: &mut Serializer) -> Result<(),SavefileError> {
        panic!("Something is wrong with version-specification of fields - there was an attempt to actually serialize a removed field!");
    }
}
impl<T: WithSchema + Deserialize> Deserialize for Removed<T> {
    fn deserialize(deserializer: &mut Deserializer) -> Result<Self,SavefileError> {
        T::deserialize(deserializer)?;
        Ok(Removed {
            phantom: std::marker::PhantomData,
        })
    }
}


impl<T> WithSchema for std::marker::PhantomData<T> {
    fn schema(_version:u32) -> Schema {
        Schema::ZeroSize
    }    
}


impl<T> Serialize for std::marker::PhantomData<T> {
    fn serialize(&self, _serializer: &mut Serializer) -> Result<(),SavefileError> {        
        Ok(())
    }
}
impl<T> Deserialize for std::marker::PhantomData<T> {
    fn deserialize(_deserializer: &mut Deserializer) -> Result<Self,SavefileError> {        
        Ok(std::marker::PhantomData)
    }
}



impl<T:WithSchema> WithSchema for Option<T> {fn schema(version:u32) -> Schema {Schema::SchemaOption(Box::new(T::schema(version)))}}

impl<T: Serialize> Serialize for Option<T> {
    fn serialize(&self, serializer: &mut Serializer) -> Result<(),SavefileError> {
        match self {
            &Some(ref x) => {serializer.write_bool(true)?;x.serialize(serializer)},
            &None => serializer.write_bool(false)
        }
    }
}
impl<T: Deserialize> Deserialize for Option<T> {
    fn deserialize(deserializer: &mut Deserializer) -> Result<Self,SavefileError> {
        let issome=deserializer.read_bool()?;
        if issome {
            Ok(Some(T::deserialize(deserializer)?))
        } else {
            Ok(None)
        }
    }
}

impl WithSchema for BitVec {
    fn schema(version:u32) -> Schema {
        Schema::Struct(SchemaStruct{
            dbg_name : "BitVec".to_string(),
            fields : vec![
                Field {
                    name: "num_bits".to_string(),
                    value: Box::new(usize::schema(version)),
                },
                Field {
                    name: "num_bytes".to_string(),
                    value: Box::new(usize::schema(version)),
                },
                Field {
                    name: "buffer".to_string(),
                    value: Box::new(Schema::Vector(
                        Box::new(u8::schema(version))
                    )),
                },
            ]
        })
    }
}

impl Serialize for BitVec {
    fn serialize(&self, serializer: &mut Serializer) -> Result<(),SavefileError> {
        let l = self.len();
        serializer.write_usize(l)?;
        let bytes=self.to_bytes();
        serializer.write_usize(bytes.len())?;
        serializer.write_bytes(&bytes)?;
        Ok(())            
    }
}
impl Deserialize for BitVec {
    fn deserialize(deserializer: &mut Deserializer) -> Result<Self,SavefileError> {
        let numbits = deserializer.read_usize()?;
        let numbytes= deserializer.read_usize()?;
        let bytes = deserializer.read_bytes(numbytes)?;
        let mut ret=BitVec::from_bytes(&bytes);
        ret.truncate(numbits);
        Ok(ret)
    }
}
    


impl<T: WithSchema> WithSchema for BinaryHeap<T> {
    fn schema(version:u32) -> Schema {
        Schema::Vector(Box::new(T::schema(version)))
    }
}
impl<T: Serialize+Ord> Serialize for BinaryHeap<T> {
    fn serialize(&self, serializer: &mut Serializer) -> Result<(),SavefileError> {
        let l = self.len();
        serializer.write_usize(l)?;
        for item in self.iter() {
            item.serialize(serializer)?
        }
        Ok(())            
    }
}
impl<T: Deserialize+Ord> Deserialize for BinaryHeap<T> {
    fn deserialize(deserializer: &mut Deserializer) -> Result<Self,SavefileError> {
        let l = deserializer.read_usize()?;
        let mut ret = BinaryHeap::with_capacity(l);
        for _ in 0..l {
            ret.push(T::deserialize(deserializer)?);
        }
        Ok(ret)
    }
}
    



fn regular_serialize_vec<T: Serialize>(item: &[T], serializer: &mut Serializer) -> Result<(),SavefileError> {
    let l = item.len();
    serializer.write_usize(l)?;
    for item in item.iter() {
        item.serialize(serializer)?
    }
    Ok(())
}

impl<T: WithSchema> WithSchema for Vec<T> {
    fn schema(version:u32) -> Schema {
        Schema::Vector(Box::new(T::schema(version)))
    }
}

impl<T: Serialize> Serialize for Vec<T> {
    default fn serialize(&self, serializer: &mut Serializer) -> Result<(),SavefileError> {
        regular_serialize_vec(self, serializer)
    }
}

impl<T: Serialize + ReprC> Serialize for Vec<T> {
    fn serialize(&self, serializer: &mut Serializer) -> Result<(),SavefileError> {
        unsafe {
            if !T::repr_c_optimization_safe(serializer.version) {
                regular_serialize_vec(self, serializer)
            } else {
                let l = self.len();
                serializer.write_usize(l)?;
                serializer.write_buf(std::slice::from_raw_parts(
                    self.as_ptr() as *const u8,
                    std::mem::size_of::<T>() * l,
                ))
            }
        }
    }
}

fn regular_deserialize_vec<T: Deserialize>(deserializer: &mut Deserializer) -> Result<Vec<T>,SavefileError> {
    let l = deserializer.read_usize()?;
    let mut ret = Vec::with_capacity(l);
    for _ in 0..l {
        ret.push(T::deserialize(deserializer)?);
    }
    Ok(ret)
}

impl<T: Deserialize> Deserialize for Vec<T> {
    default fn deserialize(deserializer: &mut Deserializer) -> Result<Self,SavefileError> {
        Ok(regular_deserialize_vec::<T>(deserializer)?)
    }
}

impl<T: Deserialize + ReprC> Deserialize for Vec<T> {
    fn deserialize(deserializer: &mut Deserializer) -> Result<Self,SavefileError> {
        if !T::repr_c_optimization_safe(deserializer.file_version) {
            Ok(regular_deserialize_vec::<T>(deserializer)?)
        } else {
            use std::mem;
            use std::heap::Alloc;
            let align = mem::align_of::<T>();
            let elem_size = mem::size_of::<T>();
            let num_elems = deserializer.read_usize()?;
            let num_bytes = elem_size * num_elems;
            let layout = if let Some(layout) = alloc::allocator::Layout::from_size_align(num_bytes, align) {
                Ok(layout)
            } else {
                Err(SavefileError::MemoryAllocationLayoutError)
            }?;
            let ptr = unsafe { alloc::heap::Heap.alloc(layout.clone())? };

            {
                let slice = unsafe { std::slice::from_raw_parts_mut(ptr, num_bytes) };
                match deserializer.reader.read_exact(slice) {
                    Ok(()) => {Ok(())}
                    Err(err) => {
                        unsafe {
                            alloc::heap::Heap.dealloc(ptr, layout);
                        }
                        Err(err)
                    }
                }?;
            }
            let ret=unsafe { Vec::from_raw_parts(ptr as *mut T, num_elems, num_elems) };
            Ok(ret)
        }
    }
}
    
unsafe impl ReprC for bool {fn repr_c_optimization_safe(_version:u32) -> bool {false}} //Hard to know if bool will always be represented by single byte. Seems to depend on a lot of stuff.
unsafe impl ReprC for u8 {fn repr_c_optimization_safe(_version:u32) -> bool {true}}
unsafe impl ReprC for i8 {fn repr_c_optimization_safe(_version:u32) -> bool {true}}
unsafe impl ReprC for u16 {fn repr_c_optimization_safe(_version:u32) -> bool {true}}
unsafe impl ReprC for i16 {fn repr_c_optimization_safe(_version:u32) -> bool {true}}
unsafe impl ReprC for u32 {fn repr_c_optimization_safe(_version:u32) -> bool {true}}
unsafe impl ReprC for i32 {fn repr_c_optimization_safe(_version:u32) -> bool {true}}
unsafe impl ReprC for u64 {fn repr_c_optimization_safe(_version:u32) -> bool {true}}
unsafe impl ReprC for i64 {fn repr_c_optimization_safe(_version:u32) -> bool {true}}
unsafe impl ReprC for usize {fn repr_c_optimization_safe(_version:u32) -> bool {true}}
unsafe impl ReprC for isize {fn repr_c_optimization_safe(_version:u32) -> bool {true}}


impl<T1> WithSchema for [T1;0] {
    fn schema(_version:u32) -> Schema {
        Schema::ZeroSize
    }
}
impl<T1> Serialize for [T1;0] {
    fn serialize(&self, _serializer: &mut Serializer) -> Result<(),SavefileError> {        
        Ok(())
    }
}
impl<T1> Deserialize for [T1;0] {
    fn deserialize(_deserializer: &mut Deserializer) -> Result<Self,SavefileError> {
        Ok([])
    }
}



impl<T1:WithSchema> WithSchema for [T1;1] {
    fn schema(version:u32) -> Schema {
        Schema::new_tuple1::<T1>(version)
    }
}
impl<T1:Serialize> Serialize for [T1;1] {
    fn serialize(&self, serializer: &mut Serializer) -> Result<(),SavefileError> {
        self[0].serialize(serializer)
    }
}
impl<T1:Deserialize> Deserialize for [T1;1] {
    fn deserialize(deserializer: &mut Deserializer) -> Result<Self,SavefileError> {
        Ok(
            [T1::deserialize(deserializer)?]
        )
    }
}

impl<T1:WithSchema> WithSchema for [T1;2] {
    fn schema(version:u32) -> Schema {
        Schema::new_tuple2::<T1,T1>(version)
    }
}
impl<T1:Serialize> Serialize for [T1;2] {
    fn serialize(&self, serializer: &mut Serializer) -> Result<(),SavefileError> {
        self[0].serialize(serializer)?;
        self[1].serialize(serializer)?;
        Ok(())
    }
}
impl<T1:Deserialize> Deserialize for [T1;2] {
    fn deserialize(deserializer: &mut Deserializer) -> Result<Self,SavefileError> {
        Ok(
            [T1::deserialize(deserializer)?,
             T1::deserialize(deserializer)?]
        )
    }
}

impl<T1:WithSchema> WithSchema for [T1;3] {
    fn schema(version:u32) -> Schema {
        Schema::new_tuple3::<T1,T1,T1>(version)
    }
}
impl<T1:Serialize> Serialize for [T1;3] {
    fn serialize(&self, serializer: &mut Serializer) -> Result<(),SavefileError> {
        self[0].serialize(serializer)?;
        self[1].serialize(serializer)?;
        self[2].serialize(serializer)?;
        Ok(())

    }
}
impl<T1:Deserialize> Deserialize for [T1;3] {
    fn deserialize(deserializer: &mut Deserializer) -> Result<Self,SavefileError> {
        Ok(
            [T1::deserialize(deserializer)?,
             T1::deserialize(deserializer)?,
             T1::deserialize(deserializer)?,]
        )
    }
}



impl<T1:WithSchema,T2:WithSchema,T3:WithSchema> WithSchema for (T1,T2,T3) {
    fn schema(version:u32) -> Schema {
        Schema::new_tuple3::<T1,T2,T3>(version)
    }
}
impl<T1:Serialize,T2:Serialize,T3:Serialize> Serialize for (T1,T2,T3) {
    fn serialize(&self, serializer: &mut Serializer) -> Result<(),SavefileError> {
        self.0.serialize(serializer)?;
        self.1.serialize(serializer)?;
        self.2.serialize(serializer)
    }
}
impl<T1:Deserialize,T2:Deserialize,T3:Deserialize> Deserialize for (T1,T2,T3) {
    fn deserialize(deserializer: &mut Deserializer) -> Result<Self,SavefileError> {
        Ok(
            (T1::deserialize(deserializer)?,
             T2::deserialize(deserializer)?,
             T3::deserialize(deserializer)?
             )
        )
    }
}



impl<T1:WithSchema,T2:WithSchema> WithSchema for (T1,T2) {
    fn schema(version:u32) -> Schema {
        Schema::new_tuple2::<T1,T2>(version)
    }
}
impl<T1:Serialize,T2:Serialize> Serialize for (T1,T2) {
    fn serialize(&self, serializer: &mut Serializer) -> Result<(),SavefileError> {
        self.0.serialize(serializer)?;
        self.1.serialize(serializer)
    }
}
impl<T1:Deserialize,T2:Deserialize> Deserialize for (T1,T2) {
    fn deserialize(deserializer: &mut Deserializer) -> Result<Self,SavefileError> {
        Ok(
            (T1::deserialize(deserializer)?,
             T2::deserialize(deserializer)?)
        )
    }
}

impl<T1:WithSchema> WithSchema for (T1,) {
    fn schema(version:u32) -> Schema {
        Schema::new_tuple1::<T1>(version)
    }
}
impl<T1:Serialize> Serialize for (T1,) {
    fn serialize(&self, serializer: &mut Serializer) -> Result<(),SavefileError> {
        self.0.serialize(serializer)
    }
}
impl<T1:Deserialize> Deserialize for (T1,) {
    fn deserialize(deserializer: &mut Deserializer) -> Result<Self,SavefileError> {
        Ok(
            (T1::deserialize(deserializer)?,)
        )
    }
}


impl<T:arrayvec::Array<Item = u8> > WithSchema for arrayvec::ArrayString<T> {
    fn schema(_version:u32) -> Schema {
        Schema::Primitive(SchemaPrimitive::schema_string)
    }
}
impl<T:arrayvec::Array<Item = u8> > Serialize for arrayvec::ArrayString<T> {
    fn serialize(&self, serializer: &mut Serializer) -> Result<(),SavefileError> {        
        serializer.write_string(self.as_str())
    }
}
impl<T:arrayvec::Array<Item = u8> > Deserialize for arrayvec::ArrayString<T> {
    fn deserialize(deserializer: &mut Deserializer) -> Result<Self,SavefileError> {
        let s = deserializer.read_string()?;
        Ok(arrayvec::ArrayString::from(&s)?)
    }
}

use std::ops::Deref;
impl<T:WithSchema> WithSchema for Box<T> {
    fn schema(version:u32) -> Schema {
        T::schema(version)
    }
}
impl<T:Serialize> Serialize for Box<T> {
    fn serialize(&self, serializer: &mut Serializer) -> Result<(),SavefileError> {        
        self.deref().serialize(serializer)
    }
}
impl<T:Deserialize> Deserialize for Box<T> {
    fn deserialize(deserializer: &mut Deserializer) -> Result<Self,SavefileError> {
        Ok(Box::new(T::deserialize(deserializer)?))
    }
}

use std::rc::Rc;

impl<T:WithSchema> WithSchema for Rc<T> {
    fn schema(version:u32) -> Schema {
        T::schema(version)
    }
}
impl<T:Serialize> Serialize for Rc<T> {
    fn serialize(&self, serializer: &mut Serializer) -> Result<(),SavefileError> {        
        self.deref().serialize(serializer)
    }
}
impl<T:Deserialize> Deserialize for Rc<T> {
    fn deserialize(deserializer: &mut Deserializer) -> Result<Self,SavefileError> {
        Ok(Rc::new(T::deserialize(deserializer)?))
    }
}


use std::sync::Arc;

impl<T:WithSchema> WithSchema for Arc<T> {
    fn schema(version:u32) -> Schema {
        T::schema(version)
    }
}
impl<T:Serialize> Serialize for Arc<T> {
    fn serialize(&self, serializer: &mut Serializer) -> Result<(),SavefileError> {        
        self.deref().serialize(serializer)
    }
}
impl<T:Deserialize> Deserialize for Arc<T> {
    fn deserialize(deserializer: &mut Deserializer) -> Result<Self,SavefileError> {
        Ok(Arc::new(T::deserialize(deserializer)?))
    }
}

use std::cell::RefCell;
use std::cell::Cell;

impl<T:WithSchema> WithSchema for RefCell<T> {
    fn schema(version:u32) -> Schema {
        T::schema(version)
    }
}
impl<T:Serialize> Serialize for RefCell<T> {
    fn serialize(&self, serializer: &mut Serializer) -> Result<(),SavefileError> {        
        self.borrow().serialize(serializer)
    }
}
impl<T:Deserialize> Deserialize for RefCell<T> {
    fn deserialize(deserializer: &mut Deserializer) -> Result<Self,SavefileError> {
        Ok(RefCell::new(T::deserialize(deserializer)?))
    }
}


impl<T:WithSchema> WithSchema for Cell<T> {
    fn schema(version:u32) -> Schema {
        T::schema(version)
    }
}
impl<T:Serialize+Copy> Serialize for Cell<T> {
    fn serialize(&self, serializer: &mut Serializer) -> Result<(),SavefileError> {        
        let t:T = self.get();
        t.serialize(serializer)
    }
}
impl<T:Deserialize> Deserialize for Cell<T> {
    fn deserialize(deserializer: &mut Deserializer) -> Result<Self,SavefileError> {
        Ok(Cell::new(T::deserialize(deserializer)?))
    }
}



impl WithSchema for () {
    fn schema(_version:u32) -> Schema {
        Schema::ZeroSize
    }
}
impl Serialize for () {
    fn serialize(&self, _serializer: &mut Serializer) -> Result<(),SavefileError> {        
        Ok(())
    }
}
impl Deserialize for () {
    fn deserialize(_deserializer: &mut Deserializer) -> Result<Self,SavefileError> {
        Ok(())
    }
}



impl WithSchema for bool {fn schema(_version:u32) -> Schema {Schema::Primitive(SchemaPrimitive::schema_bool)}}
impl WithSchema for u8 {fn schema(_version:u32) -> Schema {Schema::Primitive(SchemaPrimitive::schema_u8)}}
impl WithSchema for i8 {fn schema(_version:u32) -> Schema {Schema::Primitive(SchemaPrimitive::schema_i8)}}
impl WithSchema for u16 {fn schema(_version:u32) -> Schema {Schema::Primitive(SchemaPrimitive::schema_u16)}}
impl WithSchema for i16 {fn schema(_version:u32) -> Schema {Schema::Primitive(SchemaPrimitive::schema_i16)}}
impl WithSchema for u32 {fn schema(_version:u32) -> Schema {Schema::Primitive(SchemaPrimitive::schema_u32)}}
impl WithSchema for i32 {fn schema(_version:u32) -> Schema {Schema::Primitive(SchemaPrimitive::schema_i32)}}
impl WithSchema for u64 {fn schema(_version:u32) -> Schema {Schema::Primitive(SchemaPrimitive::schema_u64)}}
impl WithSchema for i64 {fn schema(_version:u32) -> Schema {Schema::Primitive(SchemaPrimitive::schema_i64)}}
impl WithSchema for usize {fn schema(_version:u32) -> Schema {
        match std::mem::size_of::<usize>() {
            4 => Schema::Primitive(SchemaPrimitive::schema_u32),
            8 => Schema::Primitive(SchemaPrimitive::schema_u64),
            _ => panic!("Size of usize was neither 32 bit or 64 bit. This is not supported by the savefile crate."),
        }
}}
impl WithSchema for isize {fn schema(_version:u32) -> Schema {
        match std::mem::size_of::<isize>() {
            4 => Schema::Primitive(SchemaPrimitive::schema_i32),
            8 => Schema::Primitive(SchemaPrimitive::schema_i64),
            _ => panic!("Size of isize was neither 32 bit or 64 bit. This is not supported by the savefile crate."),
        }
}}
impl WithSchema for f32 {fn schema(_version:u32) -> Schema {Schema::Primitive(SchemaPrimitive::schema_f32)}}
impl WithSchema for f64 {fn schema(_version:u32) -> Schema {Schema::Primitive(SchemaPrimitive::schema_f64)}}

impl Serialize for u8 {
    fn serialize(&self, serializer: &mut Serializer) -> Result<(),SavefileError> {
        serializer.write_u8(*self)
    }
}
impl Deserialize for u8 {
    fn deserialize(deserializer: &mut Deserializer) -> Result<Self,SavefileError> {
        deserializer.read_u8()
    }
}
impl Serialize for bool {
    fn serialize(&self, serializer: &mut Serializer) -> Result<(),SavefileError> {
        serializer.write_bool(*self)
    }
}
impl Deserialize for bool {
    fn deserialize(deserializer: &mut Deserializer) -> Result<Self,SavefileError> {
        deserializer.read_bool()
    }
}
impl Serialize for i8 {
    fn serialize(&self, serializer: &mut Serializer) -> Result<(),SavefileError> {
        serializer.write_i8(*self)
    }
}
impl Deserialize for i8 {
    fn deserialize(deserializer: &mut Deserializer) -> Result<Self,SavefileError> {
        deserializer.read_i8()
    }
}


impl Serialize for f32 {
    fn serialize(&self, serializer: &mut Serializer) -> Result<(),SavefileError> {
        serializer.write_f32(*self)
    }
}
impl Deserialize for f32 {
    fn deserialize(deserializer: &mut Deserializer) -> Result<Self,SavefileError> {
        deserializer.read_f32()
    }
}

impl Serialize for f64 {
    fn serialize(&self, serializer: &mut Serializer) -> Result<(),SavefileError> {
        serializer.write_f64(*self)
    }
}
impl Deserialize for f64 {
    fn deserialize(deserializer: &mut Deserializer) -> Result<Self,SavefileError> {
        deserializer.read_f64()
    }
}


impl Serialize for u16 {
    fn serialize(&self, serializer: &mut Serializer) -> Result<(),SavefileError> {
        serializer.write_u16(*self)
    }
}
impl Deserialize for u16 {
    fn deserialize(deserializer: &mut Deserializer) -> Result<Self,SavefileError> {
        deserializer.read_u16()
    }
}
impl Serialize for i16 {
    fn serialize(&self, serializer: &mut Serializer) -> Result<(),SavefileError> {
        serializer.write_i16(*self)
    }
}
impl Deserialize for i16 {
    fn deserialize(deserializer: &mut Deserializer) -> Result<Self,SavefileError> {
        deserializer.read_i16()
    }
}

impl Serialize for u32 {
    fn serialize(&self, serializer: &mut Serializer) -> Result<(),SavefileError> {
        serializer.write_u32(*self)
    }
}
impl Deserialize for u32 {
    fn deserialize(deserializer: &mut Deserializer) -> Result<Self,SavefileError> {
        deserializer.read_u32()
    }
}
impl Serialize for i32 {
    fn serialize(&self, serializer: &mut Serializer) -> Result<(),SavefileError> {
        serializer.write_i32(*self)
    }
}
impl Deserialize for i32 {
    fn deserialize(deserializer: &mut Deserializer) -> Result<Self,SavefileError> {
        deserializer.read_i32()
    }
}

impl Serialize for u64 {
    fn serialize(&self, serializer: &mut Serializer) -> Result<(),SavefileError> {
        serializer.write_u64(*self)
    }
}
impl Deserialize for u64 {
    fn deserialize(deserializer: &mut Deserializer) -> Result<Self,SavefileError> {
        deserializer.read_u64()
    }
}
impl Serialize for i64 {
    fn serialize(&self, serializer: &mut Serializer) -> Result<(),SavefileError> {
        serializer.write_i64(*self)
    }
}
impl Deserialize for i64 {
    fn deserialize(deserializer: &mut Deserializer) -> Result<Self,SavefileError> {
        deserializer.read_i64()
    }
}

impl Serialize for usize {
    fn serialize(&self, serializer: &mut Serializer) -> Result<(),SavefileError> {
        serializer.write_usize(*self)
    }
}
impl Deserialize for usize {
    fn deserialize(deserializer: &mut Deserializer) -> Result<Self,SavefileError> {
        deserializer.read_usize()
    }
}
impl Serialize for isize {
    fn serialize(&self, serializer: &mut Serializer) -> Result<(),SavefileError> {
        serializer.write_isize(*self)
    }
}
impl Deserialize for isize {
    fn deserialize(deserializer: &mut Deserializer) -> Result<Self,SavefileError> {
        deserializer.read_isize()
    }
}
