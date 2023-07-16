use roc_collections::VecSet;
use roc_types::subs::{Subs, Variable};

pub struct Env<'a> {
    subs: &'a mut Subs,
    seen_recursion: VecSet<(Variable, Variable)>,
    fixed_variables: VecSet<Variable>,
}

impl std::ops::Deref for Env<'_> {
    type Target = Subs;

    fn deref(&self) -> &Self::Target {
        self.subs
    }
}

impl std::ops::DerefMut for Env<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.subs
    }
}

impl<'a> Env<'a> {
    pub fn new(subs: &'a mut Subs) -> Self {
        Self {
            subs,
            seen_recursion: Default::default(),
            fixed_variables: Default::default(),
        }
    }

    pub fn add_recursion_pair(&mut self, var1: Variable, var2: Variable) {
        let pair = (
            self.subs.get_root_key_without_compacting(var1),
            self.subs.get_root_key_without_compacting(var2),
        );

        let already_seen = self.seen_recursion.insert(pair);
        debug_assert!(!already_seen);
    }

    pub fn remove_recursion_pair(&mut self, var1: Variable, var2: Variable) {
        #[cfg(debug_assertions)]
        let size_before = self.seen_recursion.len();

        self.seen_recursion.retain(|(v1, v2)| {
            let is_recursion_pair = self.subs.equivalent_without_compacting(*v1, var1)
                && self.subs.equivalent_without_compacting(*v2, var2);
            !is_recursion_pair
        });

        #[cfg(debug_assertions)]
        let size_after = self.seen_recursion.len();

        #[cfg(debug_assertions)]
        debug_assert!(size_after < size_before, "nothing was removed");
    }

    pub fn seen_recursion_pair(&self, var1: Variable, var2: Variable) -> bool {
        let (var1, var2) = (
            self.subs.get_root_key_without_compacting(var1),
            self.subs.get_root_key_without_compacting(var2),
        );

        self.seen_recursion.contains(&(var1, var2))
    }

    pub fn was_fixed(&self, var: Variable) -> bool {
        self.fixed_variables
            .iter()
            .any(|fixed_var| self.subs.equivalent_without_compacting(*fixed_var, var))
    }

    pub fn extend_fixed_variables(&mut self, vars: impl IntoIterator<Item = Variable>) {
        self.fixed_variables.extend(vars);
    }
}
