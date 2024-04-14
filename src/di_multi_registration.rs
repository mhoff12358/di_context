use godot::prelude::*;

use crate::multi_registration::get_canonical_name;

use super::di_context::DiContext;

#[derive(GodotClass)]
#[class(base=Node)]
pub struct DiMultiRegistration {
    #[export]
    type_name: GString,

    #[export]
    remove_registration_object: bool,

    #[export]
    register_into_own_context: bool,

    base: Base<Node>,
}

#[godot_api]
impl DiMultiRegistration {
    pub fn multi_register(
        node_to_register: &Gd<Node>,
        type_name: &GString,
        register_into_own_context: bool,
    ) {
        let context = if register_into_own_context {
            DiContext::get_nearest(node_to_register)
        } else {
            DiContext::get_nearest_exclude_self(node_to_register)
        };
        if let Some(mut context) = context {
            let type_name = if type_name.chars_checked().is_empty() {
                node_to_register.get_class()
            } else {
                type_name.clone()
            };
            context
                .bind_mut()
                .multiregister(node_to_register.clone(), type_name);
        } else {
            godot_print!("Tried to register a node with no context in its parentage.");
        }
    }
}

#[godot_api]
impl INode for DiMultiRegistration {
    fn init(base: godot::obj::Base<Self::Base>) -> Self {
        Self {
            type_name: "".into(),
            remove_registration_object: false,
            register_into_own_context: false,
            base,
        }
    }

    fn enter_tree(&mut self) {
        let parent = self.base().get_parent().unwrap();
        DiMultiRegistration::multi_register(
            &parent,
            &self.type_name,
            self.register_into_own_context,
        );
        self.to_gd().queue_free();
    }
}
