# android.thermal_status

## 能力

查询当前 Android 设备的热状态信息，包括各传感器温度、散热等级等只读数据。

## 允许读取

- CPU / GPU / 电池温度
- 热节流等级（throttling level）
- 散热状态枚举值

## 不允许

- 修改散热策略或温控参数
- 访问内核热管理接口
- 执行任意命令

## 风险等级

low

## 调用示例

```json
{
  "tool": "android.thermal_status",
  "arguments": {}
}
```
