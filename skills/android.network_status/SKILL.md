# android.network_status

## 能力

查询当前 Android 设备的网络连接状态，包括 Wi-Fi、移动数据、IP 地址等只读信息。

## 允许读取

- Wi-Fi 连接状态与 SSID
- 移动数据开关状态
- 当前 IP 地址（本地）
- 网络类型（Wi-Fi / 4G / 5G）

## 不允许

- 修改网络设置或切换网络
- 访问其他设备的网络信息
- 执行网络请求

## 风险等级

low

## 调用示例

```json
{
  "tool": "android.network_status",
  "arguments": {}
}
```
