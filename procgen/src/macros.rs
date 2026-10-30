//! Macros shared by the generators' settings.

/// Copy accessors for scalar settings: `get_x: x, f64,` defines `pub fn get_x(&self) -> f64`.
macro_rules! settings_scalar_accessors {
    ($ ( $name: ident : $field: ident, $type: ty, )* ) => {
    $(
        pub fn $name(&self) -> $type {
            self.$field
        }
    )*
    }
}
