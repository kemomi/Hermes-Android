# android.app.list

## 能力

列出当前 Android 设备上已安装的应用程序，包括包名、版本、是否系统应用等只读信息。

## 允许读取

- 应用包名与显示名称
- 版本号（versionName / versionCode）
- 是否为系统应用
- 是否已启用

## 不允许

- 安装、卸载或禁用应用
- 访问应用私有数据
- 修改应用权限或设置

## 风险等级

low

## 调用示例

```json
{
  "tool": "android.app.list",
  "arguments": {}
}
```
