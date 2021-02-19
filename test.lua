function init(nstrip, nmod)
	print("Initializing with "..nstrip.." strips with "..nmod.." modules each.")

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
	print("Going round and round...")
	return red, green, blue, white
end
