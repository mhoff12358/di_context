use std::{cell::RefCell, collections::HashMap, ops::Deref};

use godot::prelude::*;

use crate::multi_registration::{get_canonical_name, MultiregistrationTrait};

thread_local! {
    static DI_REGISTRY: RefCell<HashMap<InstanceId, (Gd<Node>, Gd<DiContext>)>> =
        RefCell::new(HashMap::default());
}

fn insert_di_context<T: Inherits<Node> + GodotClass>(
    parent: impl Deref<Target = Gd<T>>,
    di_context: Gd<DiContext>,
) {
    DI_REGISTRY.with(|di_registry| {
        di_registry
            .borrow_mut()
            .insert(parent.instance_id(), (parent.clone().upcast(), di_context))
    });
}

fn lookup_di_context<T: Inherits<Node> + GodotClass>(
    parent: impl Deref<Target = Gd<T>>,
) -> Option<Gd<DiContext>> {
    let mut result = None;
    DI_REGISTRY.with(|di_registry| {
        if let Some(context) = di_registry.borrow().get(&parent.instance_id()) {
            result = Some(context.1.clone());
        }
    });
    return result;
}

fn clear_di_context_id(id: &InstanceId) {
    DI_REGISTRY.with(|di_registry| {
        di_registry.borrow_mut().remove(id);
    });
}

fn clear_di_context(context: &Gd<DiContext>) {
    let mut id_to_clear = None;
    DI_REGISTRY.with(|di_registry| {
        for (key, value) in di_registry.borrow().iter() {
            if value.1 == *context {
                id_to_clear = Some(*key);
                break;
            }
        }
    });
    if let Some(id_to_clear) = id_to_clear {
        clear_di_context_id(&id_to_clear);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct RegistrationKey {
    pub type_name: GString,
    pub id: GString,
}

#[derive(GodotClass)]
#[class(base=Resource, init)]
pub struct ReRegistration {
    #[export]
    type_name: GString,
    #[export]
    id: GString,
}

#[derive(GodotClass)]
#[class(base=Node)]
pub struct DiContext {
    parent_context: Option<Gd<DiContext>>,
    registered_nodes: HashMap<RegistrationKey, Gd<Node>>,
    multiregistered_nodes: HashMap<GString, Vec<Gd<Node>>>,

    children_to_search_for_registered_nodes: HashMap<RegistrationKey, Vec<InstanceId>>,
    children_to_search_for_multiregistered_nodes: HashMap<GString, Vec<InstanceId>>,

    #[export]
    verbose_logging_name: GString,

    #[export]
    re_multiregister_in_parent: Array<GString>,

    #[export]
    re_register_in_parent: Array<Gd<ReRegistration>>,

    base: Base<Node>,
}

impl DiContext {
    pub fn try_get_registered_node_template<T: GodotClass + Inherits<Node>>(
        &self,
        id: GString,
    ) -> Option<Gd<T>> {
        self.try_get_registered_node_with_id(T::class_name().to_gstring(), id)
            .map(|node| node.cast())
    }

    pub fn get_registered_node_template<T: GodotClass + Inherits<Node>>(
        &self,
        id: GString,
    ) -> Gd<T> {
        if let Some(result) = self
            .try_get_registered_node_with_id(T::class_name().to_gstring(), id.clone())
            .map(|node| node.cast())
        {
            result
        } else {
            panic!(
                "Failed to find node with autogenerated type name {} and id {}",
                T::class_name().to_gstring(),
                id
            );
        }
    }
}

#[godot_api]
impl DiContext {
    #[func]
    pub fn try_get_registered_node(&self, type_name: GString) -> Option<Gd<Node>> {
        return self.try_get_registered_node_with_id(type_name, "".into());
    }

    #[func]
    pub fn get_registered_node(&self, type_name: GString) -> Gd<Node> {
        if let Some(result) = self.try_get_registered_node_with_id(type_name.clone(), "".into()) {
            result
        } else {
            panic!("Failed to find node with type {}", type_name);
        }
    }

    fn try_get_registered_node_with_id_no_parent_search(
        &self,
        key: &RegistrationKey,
        child_to_ignore: Option<InstanceId>,
    ) -> Option<Gd<Node>> {
        if let Some(locally_found) = self.registered_nodes.get(&key) {
            return Some(locally_found.clone());
        } else {
            if let Some(children_to_search) = self.children_to_search_for_registered_nodes.get(&key)
            {
                for child_to_search in children_to_search.iter() {
                    if Some(*child_to_search) != child_to_ignore {
                        let child_search = Gd::<DiContext>::from_instance_id(*child_to_search)
                            .bind()
                            .try_get_registered_node_with_id_no_parent_search(key, child_to_ignore);
                        if child_search.is_some() {
                            return child_search;
                        }
                    }
                }
            }
        }
        return None;
    }

    fn try_get_registered_node_with_id_impl(
        &self,
        key: &RegistrationKey,
        child_to_ignore: Option<InstanceId>,
    ) -> Option<Gd<Node>> {
        let search_self_and_children =
            self.try_get_registered_node_with_id_no_parent_search(&key, child_to_ignore);
        if search_self_and_children.is_some() {
            return search_self_and_children;
        } else {
            if let Some(parent_context) = self.parent_context.as_ref() {
                return parent_context.bind().try_get_registered_node_with_id_impl(
                    key,
                    Some(self.base().instance_id_unchecked()),
                );
            } else {
                return None;
            }
        }
    }

    #[func]
    pub fn try_get_registered_node_with_id(
        &self,
        type_name: GString,
        id: GString,
    ) -> Option<Gd<Node>> {
        let key = RegistrationKey { type_name, id };
        return self.try_get_registered_node_with_id_impl(&key, None);
    }

    #[func]
    pub fn get_registered_node_with_id(&self, type_name: GString, id: GString) -> Gd<Node> {
        if let Some(result) = self.try_get_registered_node_with_id(type_name.clone(), id.clone()) {
            result
        } else {
            panic!("Failed to find node with type {} and id {}", type_name, id);
        }
    }

    pub fn register_with_type<T: Inherits<Node> + GodotClass>(
        &mut self,
        node: impl Deref<Target = Gd<T>>,
        type_name: GString,
        id: GString,
    ) {
        if !self.verbose_logging_name.chars_checked().is_empty() {
            godot_print!(
                "Registering node of type {} and id {} to context {}",
                type_name,
                id,
                self.verbose_logging_name
            );
        }
        self.registered_nodes
            .insert(RegistrationKey { type_name, id }, node.clone().upcast());
    }

    pub fn register<T: Inherits<Node> + GodotClass>(
        &mut self,
        node: impl Deref<Target = Gd<T>>,
        id: GString,
    ) {
        let type_name;
        let custom_lookup_method: StringName = "_di_name".into();
        let mut node2: Gd<Node> = node.clone().upcast();
        if node2.has_method(custom_lookup_method.clone()) {
            type_name = node2.call(custom_lookup_method, &[]).stringify();
        } else {
            type_name = node2.get_class();
        }
        self.register_with_type(node, type_name, id);
    }

    pub fn multiregister(&mut self, node: Gd<Node>, key: GString) {
        self.multiregistered_nodes
            .entry(key)
            .or_default()
            .push(node);
    }

    pub fn multiregister_auto_type<T: Inherits<Node> + GodotClass>(
        &mut self,
        node: impl Deref<Target = Gd<T>>,
    ) {
        self.multiregister(node.clone().upcast(), T::class_name().to_gstring());
    }

    fn get_all_without_searching_parent<T: Inherits<Node> + GodotClass>(
        &self,
        results: &mut Vec<Gd<T>>,
        excluded_child: Option<InstanceId>,
    ) {
        let class_name = &T::class_name().to_gstring();
        if let Some(self_nodes) = self.multiregistered_nodes.get(class_name) {
            results.extend(self_nodes.iter().map(|node| node.clone().cast::<T>()));
        }
        if let Some(children_to_check) = self
            .children_to_search_for_multiregistered_nodes
            .get(class_name)
        {
            for child in children_to_check.iter() {
                if Some(*child) != excluded_child {
                    Gd::<DiContext>::from_instance_id(*child)
                        .bind()
                        .get_all_without_searching_parent::<T>(results, None);
                }
            }
        }
    }

    fn get_all_impl<T: Inherits<Node> + GodotClass>(
        &self,
        results: &mut Vec<Gd<T>>,
        excluded_child: Option<InstanceId>,
    ) {
        self.get_all_without_searching_parent::<T>(results, excluded_child);
        if let Some(parent_context) = self.parent_context.as_ref() {
            parent_context
                .bind()
                .get_all_impl::<T>(results, Some(self.base().instance_id_unchecked()));
        };
    }

    pub fn get_all<T: Inherits<Node> + GodotClass>(&self) -> Vec<Gd<T>> {
        let mut results = Vec::new();
        self.get_all_impl::<T>(&mut results, None);
        return results;
    }

    pub fn get_context<T: Inherits<Node> + GodotClass>(
        node: impl Deref<Target = Gd<T>>,
    ) -> Option<Gd<DiContext>> {
        lookup_di_context(node)
    }

    pub fn get_nearest<T: Inherits<Node> + GodotClass>(
        node: impl Deref<Target = Gd<T>>,
    ) -> Option<Gd<DiContext>> {
        let node2 = node.clone().upcast();
        if let Some(context) = lookup_di_context(node) {
            return Some(context);
        } else {
            if let Some(parent) = node2.get_parent() {
                return Self::get_nearest(&parent);
            } else {
                return None;
            }
        }
    }

    pub fn get_nearest_exclude_self<T: Inherits<Node> + GodotClass>(
        node: impl Deref<Target = Gd<T>>,
    ) -> Option<Gd<DiContext>> {
        Self::get_nearest(&node.clone().upcast().get_parent().unwrap())
    }

    #[func]
    pub fn register_node_of_type_node(&mut self, node: Gd<Node>, type_name: GString, id: GString) {
        self.register_with_type(&node, type_name, id);
    }

    #[func]
    pub fn register_node(&mut self, node: Gd<Node>, id: GString) {
        self.register(&node, id);
    }

    #[func]
    pub fn get_node_context(node: Gd<Node>) -> Option<Gd<DiContext>> {
        Self::get_context(&node)
    }

    #[func]
    pub fn get_nearest_to_node(node: Gd<Node>) -> Option<Gd<DiContext>> {
        Self::get_nearest(&node)
    }

    #[func]
    pub fn get_nearest_to_node_exclude_self(node: Gd<Node>) -> Option<Gd<DiContext>> {
        Self::get_nearest_exclude_self(&node)
    }

    fn add_child_reregistration(
        &mut self,
        child_id: InstanceId,
        child_reregistrations: &Array<Gd<ReRegistration>>,
        child_remultiregistrations: &Array<GString>,
    ) {
        for reregistration in child_reregistrations.iter_shared() {
            let reregistration = reregistration.bind();
            self.children_to_search_for_registered_nodes
                .entry(RegistrationKey {
                    type_name: reregistration.type_name.clone(),
                    id: reregistration.id.clone(),
                })
                .or_default()
                .push(child_id);
        }
        for type_name in child_remultiregistrations.iter_shared() {
            self.children_to_search_for_multiregistered_nodes
                .entry(type_name)
                .or_default()
                .push(child_id);
        }
    }
}

#[godot_api]
impl INode for DiContext {
    fn init(base: godot::obj::Base<Self::Base>) -> Self {
        Self {
            verbose_logging_name: "".into(),
            parent_context: None,
            children_to_search_for_multiregistered_nodes: Default::default(),
            children_to_search_for_registered_nodes: Default::default(),
            re_multiregister_in_parent: Array::new(),
            re_register_in_parent: Array::new(),
            registered_nodes: Default::default(),
            multiregistered_nodes: Default::default(),
            base,
        }
    }

    fn ready(&mut self) {}

    fn enter_tree(&mut self) {
        let instance_id = self.base().instance_id_unchecked();
        let parent = self.base().get_parent().unwrap();
        insert_di_context(&parent, self.base().clone().cast());
        self.parent_context = Self::get_nearest_exclude_self(&parent);
        if let Some(parent_context) = self.parent_context.as_mut() {
            parent_context.bind_mut().add_child_reregistration(
                instance_id,
                &self.re_register_in_parent,
                &self.re_multiregister_in_parent,
            );
        }
    }

    fn exit_tree(&mut self) {
        clear_di_context(&self.base().clone().cast());
    }
}
