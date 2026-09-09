# android.file.list

## 能力

列出指定绝对路径下的文件与目录条目，返回文件名、类型、大小等只读信息。仅允许访问沙箱白名单内的路径。

## 参数

- `path`（string，必填）：目标目录的绝对路径，必须以 `/` 开头

## 不允许

- 读取或修改文件内容
- 创建、删除或重命名文件
- 访问沙箱黑名单路径（/data/system、/system 等）

## 风险等级

medium

## 调用示例

```json
{
  "tool": "android.file.list",
  "arguments": {
    "path": "/sdcard/Download"
  }
}
```
