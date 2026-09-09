# android.app.launch

## 能力

启动指定的 Android 应用程序，通过包名触发应用的默认 Activity。

## 参数

- `package_name`（string，必填）：目标应用的包名，格式为非空白字符串

## 不允许

- 传递 Intent 附加参数或自定义 Action
- 以 root 身份启动应用
- 修改应用数据或设置

## 风险等级

low

## 调用示例

```json
{
  "tool": "android.app.launch",
  "arguments": {
    "package_name": "com.example.myapp"
  }
}
```
