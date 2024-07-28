package yn_server

import "wilkuu.xyz/yapnet/protocol"

type SeqProvider struct {
	current protocol.SeqType 
}

func NewSeqProvider() *SeqProvider {
	return &SeqProvider {
		current: 0,
	}
} 

func (s *SeqProvider) Take() protocol.SeqType{
	v := s.current 
	s.current = s.current + 1 
	return v 
} 


