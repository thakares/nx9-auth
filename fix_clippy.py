import re

# 1. permissions.rs
path = 'src/db/repository/sqlite/permissions.rs'
try:
    with open(path, 'r') as f:
        content = f.read()
    
    content = content.replace("    /// Find a permission by name.\n\n    async fn clear_for_role", "    async fn clear_for_role")
    
    with open(path, 'w') as f:
        f.write(content)
except FileNotFoundError:
    pass

# 2. traits.rs
path = 'src/db/repository/traits.rs'
try:
    with open(path, 'r') as f:
        content = f.read()
    
    content = content.replace("    async fn insert(\n", "    #[allow(clippy::too_many_arguments)]\n    async fn insert(\n")
    
    with open(path, 'w') as f:
        f.write(content)
except FileNotFoundError:
    pass

# 3. audit.rs
path = 'src/db/repository/audit.rs'
try:
    with open(path, 'r') as f:
        content = f.read()
    
    content = content.replace("pub async fn insert(\n", "#[allow(clippy::too_many_arguments)]\npub async fn insert(\n")
    
    with open(path, 'w') as f:
        f.write(content)
except FileNotFoundError:
    pass

# Wait, `src/db/repository/sqlite/audit.rs` implements `AuditRepository` trait!
path = 'src/db/repository/sqlite/audit.rs'
try:
    with open(path, 'r') as f:
        content = f.read()
    
    content = content.replace("    async fn insert(\n", "    #[allow(clippy::too_many_arguments)]\n    async fn insert(\n")
    
    with open(path, 'w') as f:
        f.write(content)
except FileNotFoundError:
    pass

