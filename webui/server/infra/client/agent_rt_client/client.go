package agent_rt_client

import (
	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"
	"webui-server/infra/client/agent_rt_client/proto"
)

var AgentRtClient proto.AgentServiceClient

func InitAgentRtClient(addr string) error {
	client, err := grpc.NewClient(addr, grpc.WithTransportCredentials(insecure.NewCredentials()))
	if err != nil {
		return err
	}
	AgentRtClient = proto.NewAgentServiceClient(client)
	return nil
}
