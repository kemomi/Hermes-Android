# android.process_list

## 能力

查询当前 Android 设备上正在运行的进程列表，包括进程名、PID、内存占用等只读信息。

## 允许读取

- 进程名称与 PID
- 内存占用（RSS）
- CPU 使用率快照
- 进程所属用户

## 不允许

- 终止或修改进程
- 访问进程内部数据
- 执行任意命令获取进程信息

## 风险等级

low

## 调用示例

```json
{
  "tool": "android.process_list",
  "arguments": {}
}
```
