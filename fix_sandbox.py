path = r"D:\myworkspace\Navis Go\src-tauri\src\extension\types.rs"
with open(path, "r", encoding="utf-8") as f:
    content = f.read()

# 修复 SandboxStub - 添加 check 方法
content = content.replace(
    "impl SandboxStub {\n    pub fn audit_recorder(&self) -> AuditRecorderStub { AuditRecorderStub }\n}",
    "impl SandboxStub {\n    pub fn audit_recorder(&self) -> AuditRecorderStub { AuditRecorderStub }\n    pub fn check(&self, _req: &crate::security::sandbox::permission::OperationRequest) -> crate::security::sandbox::permission::CheckResult { crate::security::sandbox::permission::CheckResult::Allow }\n}"
)

with open(path, "w", encoding="utf-8", newline="\n") as f:
    f.write(content)
print("Added check method to SandboxStub")
