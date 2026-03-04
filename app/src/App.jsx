import { useState } from "react";

import JsonInput from "./components/JsonInput";
import SummaryPanel from "./components/SummaryPanel";
import InputsTable from "./components/InputsTable";
import OutputsTable from "./components/OutputsTable";
import TxFlow from "./components/TxDiagram";
import WarningsPanel from "./components/WarningsPanel";
import PsbtViewer from "./components/PsbtViewer";

import "./styles.css";

function App() {

  const [result, setResult] = useState(null);

  return (
    <div className="container">

      <h1>CoinSmith PSBT Builder</h1>

      <JsonInput setResult={setResult} />

      {result && result.ok && (
        <>
          <SummaryPanel result={result} />

          <TxFlow result={result} />

          <div className="grid">
            <InputsTable inputs={result.selected_inputs} />
            <OutputsTable outputs={result.outputs} />
          </div>

          <WarningsPanel warnings={result.warnings} />

          <PsbtViewer psbt={result.psbt_base64} />
        </>
      )}

    </div>
  );
}

export default App;