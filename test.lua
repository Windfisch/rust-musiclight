function init(nstrip, nmod)
	print("Initializing with "..nstrip.." strips with "..nmod.." modules each.")

	print("Sampling rate: "..CONFIG['sampling_rate'].." Hz")

	local nled = nstrip * nmod

	red = {}
	green = {}
	blue = {}
	white = {}


	for i = 1,nled do
		red[i] = 0.8
		green[i] = 0.1
		blue[i] = 0.2
		white[i] = 0.1
	end

	return 0
end

function periodic()
	bass   = sigproc:get_energy_in_band(0, 400)
	mid    = sigproc:get_energy_in_band(400, 4000)
	treble = sigproc:get_energy_in_band(4000, 20000)

	print("Bass: "..bass.." – Mid: "..mid.." – Treble: "..treble)
	return red, green, blue, white
end
