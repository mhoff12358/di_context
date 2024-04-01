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
impl DiRegistration {}

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
        let context = if self.register_into_own_context {
            DiContext::get_nearest(&parent)
        } else {
            DiContext::get_nearest_exclude_self(&parent)
        };
        if let Some(mut context) = context {
            if self.type_name.chars_checked().is_empty() {
                context
                    .bind_mut()
                    .register_node(parent.clone(), self.id.clone());
            } else {
                context.bind_mut().register_with_type(
                    &parent,
                    self.type_name.clone(),
                    self.id.clone(),
                );
            }
        } else {
            godot_print!("Tried to register a node with no context in its parentage.");
        }
        self.to_gd().queue_free();
    }
}
