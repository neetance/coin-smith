function SummaryPanel({ result }) {

    return (
        <div className="card">

            <h3>Transaction Summary</h3>

            <p><b>Inputs:</b> {result.selected_inputs.length}</p>
            <p><b>Outputs:</b> {result.outputs.length}</p>

            <p><b>Fee:</b> {result.fee_sats} sats</p>

            <p>
                <b>Fee Rate:</b> {result.fee_rate_sat_vb.toFixed(2)} sat/vB
            </p>

            <p>
                <b>RBF:</b> {result.rbf_signaling ? "Enabled" : "Disabled"}
            </p>

            <p>
                <b>Locktime:</b> {result.locktime} ({result.locktime_type})
            </p>

        </div>
    );
}

export default SummaryPanel;