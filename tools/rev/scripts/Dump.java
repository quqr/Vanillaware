import ghidra.app.decompiler.DecompInterface;
import ghidra.app.decompiler.DecompileResults;
import ghidra.app.cmd.disassemble.DisassembleCommand;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;

public class Dump extends GhidraScript {
    @Override
    public void run() throws Exception {
        DecompInterface di = new DecompInterface();
        di.toggleCCode(true);
        di.toggleSyntaxTree(true);
        di.openProgram(currentProgram);
        for (String arg : getScriptArgs()) {
            Address address = toAddr(arg);
            Function function = getFunctionContaining(address);
            if (function == null) {
                DisassembleCommand command = new DisassembleCommand(address, null, true);
                command.applyTo(currentProgram, monitor);
                function = createFunction(address, null);
            }
            println("===== " + arg + " -> " + (function == null ? "null" : function.getName()) + " =====");
            if (function == null) {
                continue;
            }
            DecompileResults results = di.decompileFunction(function, 120, monitor);
            if (results != null && results.decompileCompleted()) {
                println(results.getDecompiledFunction().getC());
            } else {
                println("decompile failed: " + (results == null ? "null" : results.getErrorMessage()));
            }
        }
    }
}
