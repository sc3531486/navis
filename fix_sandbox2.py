path = r"D:\myworkspace\Navis Go\src-tauri\src\extension\types.rs"
with open(path, "r", encoding="utf-8") as f:
    content = f.read()

# 修复 SandboxStub check 方法 - 使用正确的 CheckResult 方法
content = content.replace(
    "pub fn check(&self, _req: &crate::security::sandbox::permission::OperationRequest) -> crate::security::sandbox::permission::CheckResult { crate::security::sandbox::permission::CheckResult::Allow }",
    "pub fn check(&self, _req: &crate::security::sandbox::permission::OperationRequest) -> crate::security::sandbox::permission::CheckResult { crate::security::sandbox::permission::CheckResult::allowed(crate::security::sandbox::permission::PermissionLevel::LightCheck) }"
)

with open(path, "w", encoding="utf-8", newline="\n") as f:
    f.write(content)
print("Fixed SandboxStub check method")
