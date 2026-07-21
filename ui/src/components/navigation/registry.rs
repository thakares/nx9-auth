use crate::routes::Route;


#[derive(Clone, Debug, PartialEq)]
pub struct NavigationItem {
    pub id: String,
    pub title: String,
    pub icon: String, // Lucide SVG path or icon name
    pub route: Route,
    pub permission: Option<String>,
    pub children: Vec<NavigationItem>,
}

#[derive(Clone)]
pub struct NavigationRegistry {
    pub sections: std::collections::BTreeMap<String, Vec<NavigationItem>>,
}

impl NavigationRegistry {
    pub fn new() -> Self {
        Self {
            sections: std::collections::BTreeMap::new(),
        }
    }

    pub fn register_section(&mut self, name: &str, items: Vec<NavigationItem>) {
        self.sections.insert(name.to_string(), items);
    }
}
