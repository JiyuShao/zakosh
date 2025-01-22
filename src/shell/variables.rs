use std::collections::HashMap;
use std::process::Command;

pub struct VariableManager {
    local_vars: HashMap<String, String>,
}

impl VariableManager {
    pub fn new() -> Self {
        Self {
            local_vars: HashMap::new(),
        }
    }

    pub fn load_theme_variables(&mut self, theme_file: &str) {
        // Move theme variable loading logic here
        let _ = Command::new("sh")
            .arg("-c")
            .arg(format!(
                r#"
                # 执行主题文件
                source {}
                
                # 输出环境变量（保持原始格式）
                env | while IFS= read -r line || [ -n "$line" ]; do
                    printf '%s\n' "$line"
                done

                echo "---ENV_VAR_END---"
                
                # 输出所有变量（保持原始格式）
                set | while IFS= read -r line || [ -n "$line" ]; do
                    printf '%s\n' "$line"
                done
                "#,
                theme_file
            ))
            .output();

        // Move variable parsing logic here...
    }

    pub fn get_vars(&self) -> &HashMap<String, String> {
        &self.local_vars
    }

    pub fn set_var(&mut self, name: String, value: String) {
        self.local_vars.insert(name, value);
    }
}
