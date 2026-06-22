//go:build !windows

package objectiveai

import (
	"os/exec"
	"syscall"
)

// applyDetach detaches the child so it outlives the parent — the Go mirror of
// the JS `detached: true` + `child.unref()` / Python `start_new_session=True`.
func applyDetach(cmd *exec.Cmd) {
	if cmd.SysProcAttr == nil {
		cmd.SysProcAttr = &syscall.SysProcAttr{}
	}
	cmd.SysProcAttr.Setsid = true
}
