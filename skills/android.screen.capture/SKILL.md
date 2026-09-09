# android.screen.capture

## 能力

截取当前 Android 设备屏幕截图，返回图像数据的 Base64 编码或临时文件路径。此操作为只读，不会修改屏幕内容。

## 允许读取

- 当前屏幕像素数据
- 屏幕分辨率与方向

## 不允许

- 模拟触摸或按键操作
- 修改屏幕显示内容
- 录制视频或持续截屏

## 风险等级

medium

## 调用示例

```json
{
  "tool": "android.screen.capture",
  "arguments": {}
}
```
