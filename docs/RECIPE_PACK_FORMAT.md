# N1 Recipe Pack v1

配方包是 UTF-8 JSON 文件，扩展名为 `.n1recipe.json`，`format` 固定为 `n1-recipe-pack`，`formatVersion` 为 `1`。正式约束见 `schema/n1-recipe-pack-v1.schema.json`。

`package.id` 应使用稳定的作者命名空间，版本使用 SemVer。每条 recipe 的 `id` 在包内必须稳定且唯一；`mode` 区分普通文本与命令组。新增可选字段保持向后兼容，提高 `formatVersion` 则要求应用明确支持。

发布前务必检查 Token、密码、Cookie、私钥、个人路径和服务器地址。配方只会作为文本导入与复制，绝不会在导入时执行。
