package biz

import (
	"context"
	"encoding/json"
	"fmt"
	"github.com/gin-gonic/gin"
	"github.com/google/uuid"
	"io"
	"webui-server/infra/client/agent_rt_client"
	"webui-server/infra/client/agent_rt_client/proto"
)

func AgentCall(c *gin.Context) {
	c.Header("Content-Type", "text/event-stream")
	c.Header("Cache-Control", "no-cache")
	c.Header("Connection", "keep-alive")

	body, err := io.ReadAll(c.Request.Body)
	if err != nil {
		c.JSON(200, gin.H{
			"code":    400,
			"message": err.Error(),
		})
		return
	}
	fmt.Println("req ---> ", string(body))
	req := new(proto.AgentServiceCallRequest)
	if err = json.Unmarshal(body, req); err != nil {
		c.JSON(200, gin.H{
			"code":    400,
			"message": err.Error(),
		})
		return
	}
	if req.TaskCode == "" {
		code, _ := uuid.NewRandom()
		req.TaskCode = code.String()
	}

	send := SSEStream(c)
	defer close(send)

	call, err := agent_rt_client.AgentRtClient.Call(context.Background(), req)
	if err != nil {
		c.JSON(200, gin.H{
			"code":    400,
			"message": err.Error(),
		})
		return
	}

	for true {
		msg, err := call.Recv()
		if err == io.EOF {
			return
		} else if err != nil {
			msg = &proto.AgentServiceCallResponse{
				Code:    500,
				Message: err.Error(),
			}
			data, _ := json.Marshal(msg)
			send <- string(data)
			return
		}
		data, _ := json.Marshal(msg)
		send <- string(data)
	}

}

func SSEStream(c *gin.Context) chan string {
	dataChan := make(chan string)
	go c.Stream(func(w io.Writer) bool {
		defer func() {
			if err := recover(); err != nil {
				fmt.Println("panice:->", err)
			}
		}()

		if s, ok := <-dataChan; ok {
			//c.SSEvent("data", s)
			w.Write([]byte(fmt.Sprintf("%s", s)))
			return true
		} else {
			return false
		}
	})
	return dataChan
}
