# android.ui.tree

## 能力

获取当前 Android 设备屏幕的 UI 层级树（Accessibility Node Tree），以结构化 JSON 形式返回控件类型、文本、边界等信息。此操作为只读。

## 允许读取

- 控件类名与资源 ID
- 控件文本内容与描述
- 控件边界矩形坐标
- 控件可点击 / 可聚焦状态

## 不允许

- 模拟点击、滑动或输入操作
- 修改控件属性或内容
- 访问应用私有数据

## 风险等级

medium

## 调用示例

```json
{
  "tool": "android.ui.tree",
  "arguments": {}
}
```
