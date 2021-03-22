# rust_build

rust build web server

### rust openssl dependence

```
sudo apt install openssl libssl-devel
```

### CHANGELOG

#### 0.4.0

- 分离邮件发送功能
- update deps
- 新增完整的 query 查询接口
- 可以忽略 scm 参数

#### 0.3.0

- `reqwest` 升级到`0.11`
- `tokio` 升级到`1.0`
- `mongodb` 配套升级

#### 0.2.2

- 支持钉钉群消息推送打包结果

#### 0.2.0

- 支持所有 args
- 支持 docker 部署
