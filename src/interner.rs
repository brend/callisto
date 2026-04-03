use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Symbol(pub u32);

#[derive(Debug, Default)]
pub struct Interner {
    map: HashMap<String, Symbol>,
    strings: Vec<String>,
}

impl Interner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn intern(&mut self, s: &str) -> Symbol {
        if let Some(symbol) = self.map.get(s) {
            return *symbol;
        }
        let symbol = Symbol(self.strings.len() as u32);
        let owned = s.to_string();
        self.strings.push(owned.clone());
        self.map.insert(owned, symbol);
        symbol
    }

    pub fn resolve(&self, symbol: Symbol) -> Option<&str> {
        self.strings.get(symbol.0 as usize).map(String::as_str)
    }
}

#[cfg(test)]
mod tests {
    use super::{Interner, Symbol};

    #[test]
    fn interning_same_string_returns_same_symbol() {
        let mut interner = Interner::new();
        let first = interner.intern("alpha");
        let second = interner.intern("alpha");
        let third = interner.intern("beta");

        assert_eq!(first, second);
        assert_ne!(first, third);
        assert_eq!(interner.resolve(first), Some("alpha"));
        assert_eq!(interner.resolve(third), Some("beta"));
    }

    #[test]
    fn resolving_unknown_symbol_returns_none() {
        let interner = Interner::new();
        assert_eq!(interner.resolve(Symbol(99)), None);
    }
}
