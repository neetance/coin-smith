function TxFlow({ result }) {

    return (
        <div className="card">

            <h3>Transaction Flow</h3>

            <div className="diagram">

                <div>

                    <h4>Inputs</h4>

                    {result.selected_inputs.map((i, idx) => (
                        <div key={idx} className="box">
                            {i.value_sats} sats
                        </div>
                    ))}

                </div>

                <div className="arrow">→</div>

                <div className="box">

                    Transaction

                    <div style={{ fontSize: "12px", color: "#aaa" }}>
                        fee: {result.fee_sats} sats
                    </div>

                </div>

                <div className="arrow">→</div>

                <div>

                    <h4>Outputs</h4>

                    {result.outputs.map((o, idx) => (
                        <div key={idx} className="box">
                            {o.value_sats} sats
                            {o.is_change && " (change)"}
                        </div>
                    ))}

                </div>

            </div>

        </div>
    );
}

export default TxFlow;