#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
#[cfg(test)]
mod wasm32 {
    use ark_std::test_rng;
    use ezkl::circuit::modules::elgamal::ElGamalVariables;
    use ezkl::circuit::modules::poseidon::spec::{PoseidonSpec, POSEIDON_RATE, POSEIDON_WIDTH};
    use ezkl::circuit::modules::poseidon::PoseidonChip;
    use ezkl::circuit::modules::Module;
    use ezkl::circuit::Tolerance;
    use ezkl::commands::RunArgs;
    use ezkl::graph::modules::POSEIDON_LEN_GRAPH;
    use ezkl::graph::GraphSettings;
    use ezkl::pfsys::Snark;
    use ezkl::wasm::{
        elgamal_decrypt_wasm, elgamal_encrypt_wasm, gen_circuit_settings_wasm, gen_pk_wasm,
        gen_vk_wasm, poseidon_hash_wasm, prove_wasm, verify_wasm,
    };
    use halo2curves::bn256::{Fr, G1Affine};
    pub use wasm_bindgen_rayon::init_thread_pool;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    pub const KZG_PARAMS: &[u8] = include_bytes!("../tests/wasm/kzg");
    pub const CIRCUIT_PARAMS: &[u8] = include_bytes!("../tests/wasm/settings.json");
    pub const VK: &[u8] = include_bytes!("../tests/wasm/test.key");
    pub const PK: &[u8] = include_bytes!("../tests/wasm/test.provekey");
    pub const WITNESS: &[u8] = include_bytes!("../tests/wasm/test.witness.json");
    pub const PROOF: &[u8] = include_bytes!("../tests/wasm/test.proof");
    pub const NETWORK: &[u8] = include_bytes!("../tests/wasm/test.onnx");

    #[wasm_bindgen_test]
    async fn verify_elgamal_wasm() {
        let mut rng = test_rng();

        let var = ElGamalVariables::gen_random(&mut rng);

        let mut message: Vec<Fr> = vec![];
        for i in 0..32 {
            message.push(Fr::from(i as u64));
        }

        let pk = serde_json::to_vec(&var.pk).unwrap();
        let message_ser = serde_json::to_vec(&message).unwrap();
        let r = serde_json::to_vec(&var.r).unwrap();

        let cipher = elgamal_encrypt_wasm(
            wasm_bindgen::Clamped(pk.clone()),
            wasm_bindgen::Clamped(message_ser.clone()),
            wasm_bindgen::Clamped(r.clone()),
        );

        let sk = serde_json::to_vec(&var.sk).unwrap();

        let decrypted_message =
            elgamal_decrypt_wasm(wasm_bindgen::Clamped(cipher), wasm_bindgen::Clamped(sk));

        let decrypted_message: Vec<Fr> = serde_json::from_slice(&decrypted_message[..]).unwrap();

        assert_eq!(message, decrypted_message)
    }

    #[wasm_bindgen_test]
    async fn verify_hash() {
        let mut message: Vec<Fr> = vec![];
        for i in 0..32 {
            message.push(Fr::from(i as u64));
        }

        let message_ser = serde_json::to_vec(&message).unwrap();

        let hash = poseidon_hash_wasm(wasm_bindgen::Clamped(message_ser));
        let hash: Vec<Vec<Fr>> = serde_json::from_slice(&hash[..]).unwrap();

        let reference_hash =
            PoseidonChip::<PoseidonSpec, POSEIDON_WIDTH, POSEIDON_RATE, POSEIDON_LEN_GRAPH>::run(
                message.clone(),
            )
            .unwrap();

        assert_eq!(hash, reference_hash)
    }

    #[wasm_bindgen_test]
    async fn verify_pass() {
        let value = verify_wasm(
            wasm_bindgen::Clamped(PROOF.to_vec()),
            wasm_bindgen::Clamped(VK.to_vec()),
            wasm_bindgen::Clamped(CIRCUIT_PARAMS.to_vec()),
            wasm_bindgen::Clamped(KZG_PARAMS.to_vec()),
        );
        assert!(value);
    }

    #[wasm_bindgen_test]
    async fn verify_fail() {
        let og_proof: Snark<Fr, G1Affine> = serde_json::from_slice(&PROOF).unwrap();

        let proof: Snark<Fr, G1Affine> = Snark {
            proof: vec![0; 32],
            protocol: og_proof.protocol,
            instances: vec![vec![Fr::from(0); 32]],
            transcript_type: ezkl::pfsys::TranscriptType::EVM,
        };
        let proof = serde_json::to_string(&proof).unwrap().into_bytes();

        let value = verify_wasm(
            wasm_bindgen::Clamped(proof),
            wasm_bindgen::Clamped(VK.to_vec()),
            wasm_bindgen::Clamped(CIRCUIT_PARAMS.to_vec()),
            wasm_bindgen::Clamped(KZG_PARAMS.to_vec()),
        );
        // should fail
        assert!(!value);
    }

    #[wasm_bindgen_test]
    async fn prove_pass() {
        // prove
        let proof = prove_wasm(
            wasm_bindgen::Clamped(WITNESS.to_vec()),
            wasm_bindgen::Clamped(PK.to_vec()),
            wasm_bindgen::Clamped(NETWORK.to_vec()),
            wasm_bindgen::Clamped(CIRCUIT_PARAMS.to_vec()),
            wasm_bindgen::Clamped(KZG_PARAMS.to_vec()),
        );
        assert!(proof.len() > 0);

        let value = verify_wasm(
            wasm_bindgen::Clamped(proof.to_vec()),
            wasm_bindgen::Clamped(VK.to_vec()),
            wasm_bindgen::Clamped(CIRCUIT_PARAMS.to_vec()),
            wasm_bindgen::Clamped(KZG_PARAMS.to_vec()),
        );
        // should not fail
        assert!(value);
    }

    #[wasm_bindgen_test]
    async fn gen_circuit_settings_test() {
        let run_args = RunArgs {
            tolerance: Tolerance::default(),
            scale: 7,
            bits: 16,
            logrows: 17,
            batch_size: 1,
            input_visibility: "private".into(),
            output_visibility: "public".into(),
            param_visibility: "private".into(),
            allocated_constraints: Some(1000), // assuming an arbitrary value here for the sake of the example
        };

        let serialized_run_args =
            serde_json::to_vec(&run_args).expect("Failed to serialize RunArgs");

        let circuit_settings_ser = gen_circuit_settings_wasm(
            wasm_bindgen::Clamped(NETWORK.to_vec()),
            wasm_bindgen::Clamped(serialized_run_args),
        );

        assert!(circuit_settings_ser.len() > 0);

        let _circuit_settings: GraphSettings =
            serde_json::from_slice(&circuit_settings_ser[..]).unwrap();
    }

    #[wasm_bindgen_test]
    async fn gen_pk_test() {
        let pk = gen_pk_wasm(
            wasm_bindgen::Clamped(NETWORK.to_vec()),
            wasm_bindgen::Clamped(KZG_PARAMS.to_vec()),
            wasm_bindgen::Clamped(CIRCUIT_PARAMS.to_vec()),
        );

        assert!(pk.len() > 0);
    }

    #[wasm_bindgen_test]
    async fn gen_vk_test() {
        let pk = gen_pk_wasm(
            wasm_bindgen::Clamped(NETWORK.to_vec()),
            wasm_bindgen::Clamped(KZG_PARAMS.to_vec()),
            wasm_bindgen::Clamped(CIRCUIT_PARAMS.to_vec()),
        );

        let vk = gen_vk_wasm(
            wasm_bindgen::Clamped(pk),
            wasm_bindgen::Clamped(CIRCUIT_PARAMS.to_vec()),
        );

        assert!(vk.len() > 0);
    }

    #[wasm_bindgen_test]
    async fn circuit_settings_is_valid_test() {
        let run_args = RunArgs {
            tolerance: Tolerance::default(),
            scale: 0,
            bits: 5,
            logrows: 7,
            batch_size: 1,
            input_visibility: "private".into(),
            output_visibility: "public".into(),
            param_visibility: "private".into(),
            allocated_constraints: Some(1000), // assuming an arbitrary value here for the sake of the example
        };

        let serialized_run_args =
            serde_json::to_vec(&run_args).expect("Failed to serialize RunArgs");

        let circuit_settings_ser = gen_circuit_settings_wasm(
            wasm_bindgen::Clamped(NETWORK.to_vec()),
            wasm_bindgen::Clamped(serialized_run_args),
        );

        assert!(circuit_settings_ser.len() > 0);

        let pk = gen_pk_wasm(
            wasm_bindgen::Clamped(NETWORK.to_vec()),
            wasm_bindgen::Clamped(KZG_PARAMS.to_vec()),
            wasm_bindgen::Clamped(circuit_settings_ser),
        );

        assert!(pk.len() > 0);
    }

    #[wasm_bindgen_test]
    async fn pk_is_valid_test() {
        let pk = gen_pk_wasm(
            wasm_bindgen::Clamped(NETWORK.to_vec()),
            wasm_bindgen::Clamped(KZG_PARAMS.to_vec()),
            wasm_bindgen::Clamped(CIRCUIT_PARAMS.to_vec()),
        );

        assert!(pk.len() > 0);

        // prove
        let proof = prove_wasm(
            wasm_bindgen::Clamped(WITNESS.to_vec()),
            wasm_bindgen::Clamped(pk.clone()),
            wasm_bindgen::Clamped(NETWORK.to_vec()),
            wasm_bindgen::Clamped(CIRCUIT_PARAMS.to_vec()),
            wasm_bindgen::Clamped(KZG_PARAMS.to_vec()),
        );
        assert!(proof.len() > 0);

        let vk = gen_vk_wasm(
            wasm_bindgen::Clamped(pk.clone()),
            wasm_bindgen::Clamped(CIRCUIT_PARAMS.to_vec()),
        );

        let value = verify_wasm(
            wasm_bindgen::Clamped(proof.to_vec()),
            wasm_bindgen::Clamped(vk),
            wasm_bindgen::Clamped(CIRCUIT_PARAMS.to_vec()),
            wasm_bindgen::Clamped(KZG_PARAMS.to_vec()),
        );
        // should not fail
        assert!(value);
    }
}
