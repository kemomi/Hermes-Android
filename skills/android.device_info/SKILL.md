# android.device_info

## 能力

查询当前 Android 设备的基础状态信息，包括型号、系统版本、序列号等只读属性。

## 允许读取

- 设备型号与制造商
- Android 系统版本与 API Level
- 设备序列号（脱敏）
- 屏幕分辨率与密度

## 不允许

- 修改任何设备设置
- 访问用户隐私数据
- 执行任意命令

## 风险等级

low

## 调用示例

```json
{
  "tool": "android.device_info",
  "arguments": {}
}
```
