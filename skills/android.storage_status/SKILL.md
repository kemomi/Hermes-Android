# android.storage_status

## 能力

查询当前 Android 设备的存储空间使用情况，包括总容量、已用空间、可用空间等只读信息。

## 允许读取

- 内部存储总容量与可用空间
- 外部存储（SD 卡）状态
- 各分区使用百分比

## 不允许

- 修改或删除任何文件
- 访问文件内容
- 执行存储清理操作

## 风险等级

low

## 调用示例

```json
{
  "tool": "android.storage_status",
  "arguments": {}
}
```
