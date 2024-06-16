# ai agent
a agent framework

# quick start

1. run python runtime use docker
```bash
docker run -itd -p 50001:50001 wdshihaoren/python_rt:16896997
```
2. run example
```bash
cd example
cargo run --bin serve
```
3. run serve
```bash
cd webui/server
go run main.go
```
4. run webui
```bash
cd webui
trunk serve 
```
5. open addr `http://127.0.0.1:8080/index.html#dev`

## about
- Architecture design and tutorials [goto](https://juejin.cn/column/7380579037112516658)
- trunk install [goto](https://trunkrs.dev/)
- upload plugin example: `webui/single_agent_plan.json`