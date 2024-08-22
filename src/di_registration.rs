use godot::prelude::*;

use super::di_context::DiContext;

#[derive(GodotClass)]
#[class(base=Node)]
pub struct DiRegistration {
    #[export]
    type_name: GString,
    #[export]
    id: GString,

    #[export]
    remove_registration_object: bool,

    #[export]
    register_into_own_context: bool,

    base: Base<Node>,
}

#[godot_api]
impl DiRegistration {
    pub fn register(
        node_to_register: &Gd<Node>,
        type_name: &GString,
        id: &GString,
        register_into_own_context: bool,
    ) {
        let context = if register_into_own_context {
            DiContext::get_nearest(node_to_register)
        } else {
            DiContext::get_nearest_exclude_self(node_to_register)
        };
        if let Some(mut context) = context {
            if type_name.chars().is_empty() {
                context
                    .bind_mut()
                    .register_node(node_to_register.clone(), id.clone());
            } else {
                context.bind_mut().register_with_type(
                    node_to_register,
                    type_name.clone(),
                    id.clone(),
                );
            }
        } else {
            godot_print!("Tried to register a node with no context in its parentage.");
        }
    }

    pub fn register_auto_type(
        node_to_register: &Gd<Node>,
        id: &GString,
        register_into_own_context: bool,
    ) {
        let context = if register_into_own_context {
            DiContext::get_nearest(node_to_register)
        } else {
            DiContext::get_nearest_exclude_self(node_to_register)
        };
        if let Some(mut context) = context {
            context
                .bind_mut()
                .register_node(node_to_register.clone(), id.clone());
        } else {
            godot_print!("Tried to register a node with no context in its parentage.");
        }
    }
}

#[godot_api]
impl INode for DiRegistration {
    fn init(base: godot::obj::Base<Self::Base>) -> Self {
        Self {
            id: "".into(),
            type_name: "".into(),
            remove_registration_object: false,
            register_into_own_context: false,
            base,
        }
    }

    fn enter_tree(&mut self) {
        let parent = self.base().get_parent().unwrap();
        DiRegistration::register(
            &parent,
            &self.type_name,
            &self.id,
            self.register_into_own_context,
        );
        self.to_gd().queue_free();
    }
}
