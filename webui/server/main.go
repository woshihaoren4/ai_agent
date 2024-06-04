package main

import (
	"webui-server/biz"

	"github.com/gin-gonic/gin"
)

func main() {
	gin.SetMode(gin.ReleaseMode)
	app := gin.Default()
	app.Use(biz.Cors())
	app.GET("/api/v1/plugin", biz.LoadPluginConfig)
	app.Run("0.0.0.0:50000")
}
