// Bounded decompilation in an isolated analysis project; game files remain unchanged.
import ghidra.app.script.GhidraScript;
import ghidra.app.decompiler.DecompInterface;
import ghidra.app.decompiler.DecompileResults;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;
import ghidra.program.model.symbol.Reference;
import java.io.PrintWriter;

public class EffectSyncReport extends GhidraScript {
    public void run() throws Exception {
        String[] args = getScriptArgs();
        if (args.length < 2) throw new IllegalArgumentException("OUTPUT RVA [RVA ...]");
        DecompInterface decompiler = new DecompInterface();
        decompiler.openProgram(currentProgram);
        try (PrintWriter out = new PrintWriter(args[0], "UTF-8")) {
            out.println("Program: " + currentProgram.getName() + " Image base: " + currentProgram.getImageBase());
            for (String rva : java.util.Arrays.copyOfRange(args, 1, args.length)) {
                Address address = currentProgram.getImageBase().add(Long.parseLong(rva, 16));
                Function function = getFunctionAt(address);
                if (function == null) {
                    disassemble(address);
                    function = createFunction(address, null);
                }
                out.println("\nRVA " + rva + " function=" + function);
                for (Reference reference : getReferencesTo(address)) {
                    Function caller = getFunctionContaining(reference.getFromAddress());
                    out.println("Reference " + reference.getFromAddress() + " " + reference.getReferenceType() + " caller=" + caller);
                }
                if (function == null) continue;
                DecompileResults result = decompiler.decompileFunction(function, 20, monitor);
                out.println(result.decompileCompleted() ? result.getDecompiledFunction().getC() : result.getErrorMessage());
                out.flush();
            }
        } finally {
            decompiler.dispose();
        }
        println("Selected effect/UI native functions exported.");
    }
}
