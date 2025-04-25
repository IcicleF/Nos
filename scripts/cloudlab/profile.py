"""Configurable c6525 experiment."""

# Import the Portal object.
import geni.portal as portal
# Import the ProtoGENI library.
import geni.rspec.pg as pg
# Emulab specific extensions.
import geni.rspec.emulab as emulab

# Create a portal context.
pc = portal.Context()

# Create a Request object to start building the RSpec.
request = pc.makeRequestRSpec()

# Parameterize.
## 1. Node count.
pc.defineParameter("nodeCount", "Number of nodes", portal.ParameterType.INTEGER, 16, longDescription="If you specifiy more than one node, LANs will be created on the two experiment links.")

## 2. Physical node type.
phys_list = [
    "c6525-100g",
    "c6525-25g",
]
pc.defineParameter("physType", "Physical node type", portal.ParameterType.STRING, phys_list[0], phys_list, longDescription="The physical node type to use for each node.")

## 3. Whether or not inter-switch links.
pc.defineParameter("sameSwitch",  "Disable inter-switch links", portal.ParameterType.BOOLEAN, False,
                    advanced=True,
                    longDescription="Sometimes you want all the nodes connected to the same switch. " +
                    "This option will ask the resource mapper to do that, although it could be impossible to find a solution.")

# Retrieve the values the user specifies during instantiation.
params = pc.bindParameters()

# Check parameter validity.
if params.nodeCount < 1:
    pc.reportError(portal.ParameterError("You must choose at least 1 node.", ["nodeCount"]))

if params.physType not in phys_list:
    pc.reportError(portal.ParameterError("You must choose one supported physical node type.", ["physType"]))

pc.verifyParameters()

# Add nodes.
nodes = []
for i in range(params.nodeCount):
    node = request.RawPC("node" + str(i))
    node.hardware_type = params.physType
    
    # Default Ubuntu 22.04 image
    node.disk_image = "urn:publicid:IDN+emulab.net+image+emulab-ops:UBUNTU22-64-STD"

    # Make a dataset for Twemcache trace on the 1st node
    if i == 0:
        bs = node.Blockstore("bs", "/dataset")
        bs.dataset = "urn:publicid:IDN+utah.cloudlab.us:dandelion-pg0+imdataset+twemcache"

    nodes.append(node)

# Install a private/public key on node0; on other nodes, just the public key.
for (i, s) in enumerate(nodes):
    s.installRootKeys(i == 0, True) 

if params.nodeCount > 1:
    # Create 25Gb interfaces for each node (first exp link in c6525-25g / 25Gb exp link in c6525-100g).
    iface0 = []
    for (i, s) in enumerate(nodes):
        iface = s.addInterface()
        iface.addAddress(pg.IPv4Address("10.0.2." + str(i + 1), "255.255.252.0"))
        iface0.append(iface)

    lan0 = request.LAN("lan0")
    for iface in iface0:
        lan0.addInterface(iface)
    if params.sameSwitch:
        lan0.setNoInterSwitchLinks()
    

    # Create 25/100Gb interfaces for each node (second exp link in c6525-25g / 100Gb exp link in c6525-100g).
    iface1 = []
    for (i, s) in enumerate(nodes):
        iface = s.addInterface()
        iface.addAddress(pg.IPv4Address("10.0.4." + str(i + 1), "255.255.252.0"))
        iface1.append(iface)

    lan1 = request.LAN("lan1")
    for iface in iface1:
        lan1.addInterface(iface)
    if params.sameSwitch:
        lan1.setNoInterSwitchLinks()

# Print the RSpec to the enclosing page.
pc.printRequestRSpec(request)
