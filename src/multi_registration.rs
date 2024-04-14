use godot::{
    builtin::GString,
    engine::Node,
    obj::{bounds::DeclUser, GodotClass, Inherits},
};

pub trait MultiregistrationTrait: Inherits<Node> + GodotClass<Declarer = DeclUser> {
    const MULTIREGISTRATION_KEY: &'static str;
}

pub struct MultiregistrationKey {
    pub key: &'static str,
}

impl MultiregistrationKey {
    pub const fn new<T: ?Sized + MultiregistrationTrait>() -> Self {
        Self {
            key: T::MULTIREGISTRATION_KEY,
        }
    }
}

pub fn get_canonical_name(name: &GString) -> &'static str {
    for key in inventory::iter::<MultiregistrationKey> {
        let comparable_key: GString = key.key.into();
        if &comparable_key == name {
            return key.key;
        }
    }
    panic!("Getting canonical name for {}", name);
}

inventory::collect!(MultiregistrationKey);

#[macro_export]
macro_rules! multi_register {
    ($registration_key:expr, $RegistrationType:ident $body:tt) => {
        pub trait $RegistrationType: ::dicontext::godot::obj::Inherits<::dicontext::godot::engine::Node> + ::dicontext::godot::obj::GodotClass<Declarer = ::dicontext::godot::obj::bounds::DeclUser>
            $body

        impl ::dicontext::multi_registration::MultiregistrationTrait for dyn $RegistrationType {
            const MULTIREGISTRATION_KEY: &'static str = $registration_key;
        }
        ::dicontext::inventory::submit! {
            ::dicontext::multi_registration::MultiregistrationKey::new::<dyn $RegistrationType>()
        }
    };
}
