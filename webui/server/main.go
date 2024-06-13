package main

import (
	"fmt"
	"webui-server/biz"
	"webui-server/infra/client/agent_rt_client"

	"github.com/gin-gonic/gin"
)

func main() {
	//init rt
	if err := agent_rt_client.InitAgentRtClient("127.0.0.1:50002"); err != nil {
		fmt.Println("grpc client init failed:", err)
		panic(err)
	}
	//create serve
	gin.SetMode(gin.ReleaseMode)
	app := gin.Default()
	app.Use(biz.Cors())
	app.GET("/api/v1/plugin", biz.LoadPluginConfig)
	app.POST("/api/v1/agent/call", biz.AgentCall)
	app.Run("0.0.0.0:50000")
}
