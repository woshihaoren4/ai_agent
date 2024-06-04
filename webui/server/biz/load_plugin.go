package biz

import (
	"encoding/json"
	"net/http"
	"os"

	"github.com/gin-gonic/gin"
)

var PATH = "./plugin"

func LoadPluginConfig(c *gin.Context) {
	path := c.Query("path")
	if path == "" {
		path = PATH
	}
	entries, err := os.ReadDir(path)
	if err != nil {
		c.String(500, err.Error())
		return
	}
	resp := map[string][]*PluginItem{}
	for _, i := range entries {
		file, err := os.ReadFile(PATH + "/" + i.Name())
		if err != nil {
			c.String(500, err.Error())
			return
		}
		pi := new(PluginItem)
		err = json.Unmarshal(file, pi)
		if err != nil {
			c.String(500, "config error:"+i.Name()+" "+err.Error())
			return
		}
		for _, i := range pi.PluginList {
			if i.Code == "" || i.Class == "" {
				continue
			}
			if _, ok := resp[i.Class]; ok {
				resp[i.Class] = append(resp[i.Class], i)
			} else {
				resp[i.Class] = []*PluginItem{i}
			}
		}
		if pi.Code != "" && pi.Class != "" {
			pi.PluginList = nil
			if _, ok := resp[pi.Class]; ok {
				resp[pi.Class] = append(resp[pi.Class], pi)
			} else {
				resp[pi.Class] = []*PluginItem{pi}
			}
		}
	}
	c.JSON(http.StatusOK, resp)
}

type PluginItem struct {
	Code        string                 `json:"code"`
	Desc        string                 `json:"desc"`
	Class       string                 `json:"class"`
	UiType      string                 `json:"ui_type"`
	InputVars   map[string]interface{} `json:"input_vars"`
	OutputVars  interface{}            `json:"output_vars"`
	ServiceType string                 `json:"service_type"`

	PluginList []*PluginItem `json:"plugin_list,omitempty"`
}
