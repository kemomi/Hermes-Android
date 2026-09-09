# android.battery_status

## 能力

查询当前 Android 设备的电池状态，包括电量百分比、充电状态、电池温度等只读信息。

## 允许读取

- 电池电量百分比
- 充电状态（充电中 / 未充电 / 充满）
- 电池温度
- 电池健康度

## 不允许

- 修改充电策略或电池设置
- 访问其他系统信息

## 风险等级

low

## 调用示例

```json
{
  "tool": "android.battery_status",
  "arguments": {}
}
```
