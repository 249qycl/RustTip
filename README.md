# RustTip
多客户服务申请协作工具

用于实验室服务器的自动调度提醒通知

[dependencies]
lettre = "0.10.0-rc.6"
regex="1.5.5"
chrono="0.4.19"
clap="2.33.0"
serde_json = { version = "1.0", default-features = false, features = ["alloc"] }
serde= { version = "1.0", features = ["derive"] }
serde_derive="1.0"
lazy_static="1.2.0"

注意事项：
* Rust守护进程资料较少，在此使用僵尸进程拦截退出信号的方式进行替代，支持主动关闭僵尸进程；
* 交叉编译采用容器环境完成
* 进程通信暂时使用的TCP
