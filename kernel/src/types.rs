#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Pid(pub u32);

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Uid(pub u32);

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Gid(pub u32);

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FMode(pub u32);

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Dev(pub u32);

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Mode(pub u16);

#[macro_export]
macro_rules! impl_conversions {
    ($newtype:ident, $primitive:ty) => {
        impl From<$newtype> for $primitive {
            #[inline(always)]
            fn from(value: $newtype) -> $primitive {
                value.0
            }
        }

        impl From<$primitive> for $newtype {
            #[inline(always)]
            fn from(raw: $primitive) -> $newtype {
                $newtype(raw)
            }
        }
    };
}

impl_conversions!(Pid, u32);
impl_conversions!(Uid, u32);
impl_conversions!(Gid, u32);
impl_conversions!(FMode, u32);
impl_conversions!(Dev, u32);

impl_conversions!(Mode, u16);
