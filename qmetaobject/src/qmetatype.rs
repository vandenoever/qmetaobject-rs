/* Copyright (C) 2018 Olivier Goffart <ogoffart@woboq.com>

Permission is hereby granted, free of charge, to any person obtaining a copy of this software and
associated documentation files (the "Software"), to deal in the Software without restriction,
including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense,
and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so,
subject to the following conditions:

The above copyright notice and this permission notice shall be included in all copies or substantial
portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT
NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES
OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN
CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
*/
use super::*;

fn register_metatype_common<T : Sized + Clone + Default>(
    name : *const std::os::raw::c_char, gadget_metaobject : *const QMetaObject) -> i32 {
    let size = std::mem::size_of::<T>() as u32;

    extern fn deleter_fn<T>(_v: Box<T>) { };
    let deleter_fn : extern fn(_v: Box<T>) = deleter_fn;

    extern fn creator_fn<T : Default + Clone>(c : *const T) -> Box<T> {
        if c.is_null() { Box::new( Default::default() ) }
        else { Box::new(unsafe { (*c).clone() }) }
    };
    let creator_fn : extern fn(c : *const T) -> Box<T> = creator_fn;

    extern fn destructor_fn<T>(ptr : *mut T) { unsafe { std::ptr::read(ptr); } };
    let destructor_fn : extern fn(ptr : *mut T) = destructor_fn;

    extern fn constructor_fn<T : Default + Clone>(dst : *mut T, c : *const T) -> *mut T {
        unsafe {
            let n = if c.is_null() {  Default::default() }
                    else { (*c).clone() };
            std::ptr::write(dst, n);
        }
        dst
    };
    let constructor_fn : extern fn(ptr : *mut T, c : *const T) -> *mut T = constructor_fn;

    unsafe {
        cpp!([name as "const char*", size as "int", deleter_fn as "QMetaType::Deleter",
              creator_fn as "QMetaType::Creator", destructor_fn as "QMetaType::Destructor",
              constructor_fn as "QMetaType::Constructor", gadget_metaobject as "const QMetaObject*"] -> i32 as "int" {
            QMetaType::TypeFlags extraFlag(gadget_metaobject ? QMetaType::IsGadget : 0);
            return QMetaType::registerType(name ? name : gadget_metaobject->className(), deleter_fn, creator_fn, destructor_fn,
                constructor_fn, size,
                QMetaType::NeedsConstruction | QMetaType::NeedsDestruction | QMetaType::MovableType | extraFlag,
                gadget_metaobject);
        })
    }
}

/// Used by the QObject custom derive to register the type as a pointer to a QObject
#[doc(hidden)]
pub fn register_metatype_qobject<T : QObject>() -> i32 {
    let metaobject = T::static_meta_object();
    unsafe {
        cpp!([metaobject as "const QMetaObject*"] -> i32 as "int" {
            return QMetaType::registerType(metaobject->className(),
                [](void*p) { delete static_cast<void**>(p); },
                [](const void*p) -> void* { using T = void*; return new T{ p ? *static_cast<const T*>(p) : nullptr}; },
                QtMetaTypePrivate::QMetaTypeFunctionHelper<void*>::Destruct,
                QtMetaTypePrivate::QMetaTypeFunctionHelper<void*>::Construct,
                sizeof(void*),
                QMetaType::MovableType | QMetaType::PointerToQObject,
                metaobject);
        })
    }
}

/// Implement this trait for type that should be known to the QMetaObject system
pub trait QMetaType : Clone + Default {
    /// Registers the type.
    ///
    /// See the Qt documentation of qRegisterMetaType()
    ///
    /// The default implementation should work for most types
    fn register(name : &str) -> i32 {
        let name = std::ffi::CString::new(name).unwrap();
        register_metatype_common::<Self>(name.as_ptr(), std::ptr::null())
    }
}

/// QGadget are automatically QMetaType
impl<T : QGadget> QMetaType for T where T: Clone + Default {
    fn register(_name : &str) -> i32 {
        //assert!(_name == T::static_meta_object().className());
        register_metatype_common::<T>(std::ptr::null(), T::static_meta_object())
    }
}

impl QMetaType for String {
    fn register(name : &str) -> i32 {
        assert!(name == "String");
        let c_name = b"String\0".as_ptr() as *const std::os::raw::c_char;
        let type_id = register_metatype_common::<String>(c_name, std::ptr::null());
        extern fn converter_fn1(_ : *const c_void, s: &String, ptr : *mut QByteArray) {
            unsafe { std::ptr::write(ptr, QByteArray::from(&*s as &str)); }
        };
        let converter_fn1: extern fn(_ : *const c_void, s: &String, ptr : *mut QByteArray) = converter_fn1;
        extern fn converter_fn2(_ : *const c_void, s: &QByteArray, ptr : *mut String) {
            unsafe { std::ptr::write(ptr, s.to_string()); }
        };
        let converter_fn2: extern fn(_ : *const c_void, s: &QByteArray, ptr : *mut String) = converter_fn2;
        extern fn converter_fn3(_ : *const c_void, s: &String, ptr : *mut QString) {
            let s : &str = &*s;
            unsafe { std::ptr::write(ptr, QString::from(&*s as &str)); }
        };
        let converter_fn3: extern fn(_ : *const c_void, s: &String, ptr : *mut QString) = converter_fn3;
        extern fn converter_fn4(_ : *const c_void, s: &QString, ptr : *mut String) {
            unsafe { std::ptr::write(ptr, s.to_string()); }
        };
        let converter_fn4: extern fn(_ : *const c_void, s: &QString, ptr : *mut String) = converter_fn4;


        unsafe { cpp!([type_id as "int",
                    converter_fn1 as "QtPrivate::AbstractConverterFunction::Converter",
                    converter_fn2 as "QtPrivate::AbstractConverterFunction::Converter",
                    converter_fn3 as "QtPrivate::AbstractConverterFunction::Converter",
                    converter_fn4 as "QtPrivate::AbstractConverterFunction::Converter"] {
            //FIXME, the ConverterFunctor are gonna be leaking
            auto c = new QtPrivate::ConverterFunctor<TraitObject, TraitObject, TraitObject>(converter_fn1);
            if (!c->registerConverter(type_id, QMetaType::QByteArray))
                delete c;
            c = new QtPrivate::ConverterFunctor<TraitObject, TraitObject, TraitObject>(converter_fn2);
            if (!c->registerConverter(QMetaType::QByteArray, type_id))
                delete c;
            c = new QtPrivate::ConverterFunctor<TraitObject, TraitObject, TraitObject>(converter_fn3);
            if (!c->registerConverter(type_id, QMetaType::QString))
                delete c;
            c = new QtPrivate::ConverterFunctor<TraitObject, TraitObject, TraitObject>(converter_fn4);
            if (!c->registerConverter(QMetaType::QString, type_id))
                delete c;
        }) };
        type_id
    }
}

macro_rules! qdeclare_builtin_metatype {
    ($name:ty => $value:expr) => {
        impl QMetaType for $name {
            fn register(name : &str) -> i32 {
                assert!(name == stringify!($name));
                $value
            }
        }
    }
}
qdeclare_builtin_metatype!{()   => 43}
qdeclare_builtin_metatype!{bool => 1}
qdeclare_builtin_metatype!{i32  => 2}
qdeclare_builtin_metatype!{u32  => 3}
qdeclare_builtin_metatype!{i64  => 4}
qdeclare_builtin_metatype!{u64  => 5}
qdeclare_builtin_metatype!{f64  => 6}
qdeclare_builtin_metatype!{i16  => 33}
qdeclare_builtin_metatype!{i8   => 34}
qdeclare_builtin_metatype!{u16  => 36}
qdeclare_builtin_metatype!{u8   => 37}
qdeclare_builtin_metatype!{f32  => 38}
//qdeclare_builtin_metatype!{"*c_void" => 31,
qdeclare_builtin_metatype!{QString => 10}
qdeclare_builtin_metatype!{QByteArray => 12}
qdeclare_builtin_metatype!{QVariant => 41}

// FIXME!  32 bit!
qdeclare_builtin_metatype!{isize  => 32}
qdeclare_builtin_metatype!{usize  => 35}

/// Internal trait used to pass or read the type in a Q_PROPERTY
///
/// Don't implement this trait, implement the QMetaType trait.
pub trait PropertyType {
    const READ_ONLY : bool;
    fn register_type(name : &str) -> i32;
    // Note: this is &mut self becauser of the lazy initialization of the QObject* for the QObject impl
    unsafe fn pass_to_qt(&mut self, a: *mut c_void);
    unsafe fn read_from_qt(a: *const c_void) -> Self;
}


impl<T : QMetaType> PropertyType for T where T : QMetaType {
    const READ_ONLY : bool = false;
    unsafe fn pass_to_qt(&mut self, a: *mut c_void) {
        let r = a as *mut Self;
        if !r.is_null() { *r = self.clone(); }
    }

    unsafe fn read_from_qt(a: *const c_void) -> Self {
        let r = a as *const Self;
        (*r).clone()
    }

    fn register_type(name : &str) -> i32 {
        <T as QMetaType>::register(name)
    }
}

