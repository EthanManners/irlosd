1. NVIDIA 595 Drivers
2. ssh GPU is now a fresh clean production ready server with no clients on it, however the goal is to be able to have a client on it tonight 
3. No need to implement kick/twitch commands as we will later create a webpage control panel to start stream. Everything speaks ICP, so new clients that need to interact with irlosd should be implemented the same way. irlosd just needs to understand ICP
4. The final result im looking for is for when the agent is done, i should be able to put my stream key in .env, launch irlosd, press start, and i should see a test screen on my kick.
5. IrlosStudio now runs headless mode via flag --headless. This fundamentally changes how irlosd is deployed. We want to avoid the stack we had before. "This replaces the production setup of Xorg, a dummy HDMI plug XFCE, and x11vnc. The target is a GPU server you reach only over SSH."
6. Be careful of token burn when going thru IrlosStudio,
