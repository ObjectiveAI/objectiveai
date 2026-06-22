//go:build windows

package objectiveai

import (
	"os/exec"
	"syscall"
)

// applyDetach detaches the child so it outlives the parent — the Go mirror of
// the JS `detached: true` + `child.unref()` / Python `creationflags=DETACHED_PROCESS`.
func applyDetach(cmd *exec.Cmd) {
	const detachedProcess = 0x00000008 // DETACHED_PROCESS
	if cmd.SysProcAttr == nil {
		cmd.SysProcAttr = &syscall.SysProcAttr{}
	}
	cmd.SysProcAttr.CreationFlags |= detachedProcess
}
