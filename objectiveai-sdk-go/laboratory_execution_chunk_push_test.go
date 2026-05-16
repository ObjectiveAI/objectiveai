package objectiveai

import (
	"fmt"
	"testing"
)

func TestLaboratoryExecutionChunkPush(t *testing.T) {
	for stream := 0; stream < 20; stream++ {
		t.Run(fmt.Sprintf("stream_%d", stream), func(t *testing.T) {
			seed := int64(stream * 1000)
			goAcc, err := GenerateLaboratoryExecutionChunk(true, seed)
			if err != nil {
				t.Fatalf("generate init: %v", err)
			}
			cffiAcc := deepCopy(t, goAcc)
			seed++

			for j := 0; j < 20; j++ {
				chunk, err := GenerateLaboratoryExecutionChunk(true, seed)
				if err != nil {
					t.Fatalf("generate chunk %d: %v", j, err)
				}
				seed++

				goAcc.Push(chunk)

				cffiMerged, err := LaboratoryExecutionChunkMerged(cffiAcc, *chunk)
				if err != nil {
					t.Fatalf("cffi merge %d: %v", j, err)
				}
				cffiAcc = *cffiMerged

				assertRoundedEqual(t, fmt.Sprintf("chunk %d", j), toMap(t, goAcc), toMap(t, cffiAcc))
			}
		})
	}
}
