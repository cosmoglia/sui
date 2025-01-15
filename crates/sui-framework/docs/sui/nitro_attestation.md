---
title: Module `sui::nitro_attestation`
---



-  [Struct `NitroAttestationDocument`](#sui_nitro_attestation_NitroAttestationDocument)
-  [Function `verify_nitro_attestation_internal`](#sui_nitro_attestation_verify_nitro_attestation_internal)
-  [Function `verify_nitro_attestation`](#sui_nitro_attestation_verify_nitro_attestation)
-  [Function `get_module_id`](#sui_nitro_attestation_get_module_id)
-  [Function `get_timestamp`](#sui_nitro_attestation_get_timestamp)
-  [Function `get_digest`](#sui_nitro_attestation_get_digest)
-  [Function `get_pcrs`](#sui_nitro_attestation_get_pcrs)
-  [Function `get_public_key`](#sui_nitro_attestation_get_public_key)
-  [Function `get_user_data`](#sui_nitro_attestation_get_user_data)
-  [Function `get_nonce`](#sui_nitro_attestation_get_nonce)


<pre><code><b>use</b> <a href="../std/ascii.md#std_ascii">std::ascii</a>;
<b>use</b> <a href="../std/bcs.md#std_bcs">std::bcs</a>;
<b>use</b> <a href="../std/option.md#std_option">std::option</a>;
<b>use</b> <a href="../std/string.md#std_string">std::string</a>;
<b>use</b> <a href="../std/vector.md#std_vector">std::vector</a>;
<b>use</b> <a href="../sui/address.md#sui_address">sui::address</a>;
<b>use</b> <a href="../sui/clock.md#sui_clock">sui::clock</a>;
<b>use</b> <a href="../sui/hex.md#sui_hex">sui::hex</a>;
<b>use</b> <a href="../sui/object.md#sui_object">sui::object</a>;
<b>use</b> <a href="../sui/transfer.md#sui_transfer">sui::transfer</a>;
<b>use</b> <a href="../sui/tx_context.md#sui_tx_context">sui::tx_context</a>;
</code></pre>



<a name="sui_nitro_attestation_NitroAttestationDocument"></a>

## Struct `NitroAttestationDocument`



<pre><code><b>public</b> <b>struct</b> <a href="../sui/nitro_attestation.md#sui_nitro_attestation_NitroAttestationDocument">NitroAttestationDocument</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>module_id: vector&lt;u8&gt;</code>
</dt>
<dd>
</dd>
<dt>
<code>timestamp: u64</code>
</dt>
<dd>
</dd>
<dt>
<code>digest: vector&lt;u8&gt;</code>
</dt>
<dd>
</dd>
<dt>
<code>pcrs: vector&lt;vector&lt;u8&gt;&gt;</code>
</dt>
<dd>
</dd>
<dt>
<code>public_key: <a href="../std/option.md#std_option_Option">std::option::Option</a>&lt;vector&lt;u8&gt;&gt;</code>
</dt>
<dd>
</dd>
<dt>
<code>user_data: <a href="../std/option.md#std_option_Option">std::option::Option</a>&lt;vector&lt;u8&gt;&gt;</code>
</dt>
<dd>
</dd>
<dt>
<code>nonce: <a href="../std/option.md#std_option_Option">std::option::Option</a>&lt;vector&lt;u8&gt;&gt;</code>
</dt>
<dd>
</dd>
</dl>


</details>

<a name="sui_nitro_attestation_verify_nitro_attestation_internal"></a>

## Function `verify_nitro_attestation_internal`

Internal native function


<pre><code><b>fun</b> <a href="../sui/nitro_attestation.md#sui_nitro_attestation_verify_nitro_attestation_internal">verify_nitro_attestation_internal</a>(attestation: &vector&lt;u8&gt;, current_timestamp: u64): <a href="../sui/nitro_attestation.md#sui_nitro_attestation_NitroAttestationDocument">sui::nitro_attestation::NitroAttestationDocument</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="../sui/nitro_attestation.md#sui_nitro_attestation_verify_nitro_attestation_internal">verify_nitro_attestation_internal</a>(
    attestation: &vector&lt;u8&gt;,
    current_timestamp: u64
): <a href="../sui/nitro_attestation.md#sui_nitro_attestation_NitroAttestationDocument">NitroAttestationDocument</a>;
</code></pre>



</details>

<a name="sui_nitro_attestation_verify_nitro_attestation"></a>

## Function `verify_nitro_attestation`

@param attestation: attesttaion documents bytes data.
@param clock: the clock object.

Returns parsed NitroAttestationDocument after verifying the attestation.


<pre><code><b>public</b> <b>fun</b> <a href="../sui/nitro_attestation.md#sui_nitro_attestation_verify_nitro_attestation">verify_nitro_attestation</a>(attestation: &vector&lt;u8&gt;, <a href="../sui/clock.md#sui_clock">clock</a>: &<a href="../sui/clock.md#sui_clock_Clock">sui::clock::Clock</a>): <a href="../sui/nitro_attestation.md#sui_nitro_attestation_NitroAttestationDocument">sui::nitro_attestation::NitroAttestationDocument</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="../sui/nitro_attestation.md#sui_nitro_attestation_verify_nitro_attestation">verify_nitro_attestation</a>(
    attestation: &vector&lt;u8&gt;,
    <a href="../sui/clock.md#sui_clock">clock</a>: &Clock
): <a href="../sui/nitro_attestation.md#sui_nitro_attestation_NitroAttestationDocument">NitroAttestationDocument</a> {
    <a href="../sui/nitro_attestation.md#sui_nitro_attestation_verify_nitro_attestation_internal">verify_nitro_attestation_internal</a>(attestation, <a href="../sui/clock.md#sui_clock_timestamp_ms">clock::timestamp_ms</a>(<a href="../sui/clock.md#sui_clock">clock</a>))
}
</code></pre>



</details>

<a name="sui_nitro_attestation_get_module_id"></a>

## Function `get_module_id`



<pre><code><b>public</b> <b>fun</b> <a href="../sui/nitro_attestation.md#sui_nitro_attestation_get_module_id">get_module_id</a>(attestation: &<a href="../sui/nitro_attestation.md#sui_nitro_attestation_NitroAttestationDocument">sui::nitro_attestation::NitroAttestationDocument</a>): vector&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="../sui/nitro_attestation.md#sui_nitro_attestation_get_module_id">get_module_id</a>(attestation: &<a href="../sui/nitro_attestation.md#sui_nitro_attestation_NitroAttestationDocument">NitroAttestationDocument</a>): vector&lt;u8&gt; {
    attestation.module_id
}
</code></pre>



</details>

<a name="sui_nitro_attestation_get_timestamp"></a>

## Function `get_timestamp`



<pre><code><b>public</b> <b>fun</b> <a href="../sui/nitro_attestation.md#sui_nitro_attestation_get_timestamp">get_timestamp</a>(attestation: &<a href="../sui/nitro_attestation.md#sui_nitro_attestation_NitroAttestationDocument">sui::nitro_attestation::NitroAttestationDocument</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="../sui/nitro_attestation.md#sui_nitro_attestation_get_timestamp">get_timestamp</a>(attestation: &<a href="../sui/nitro_attestation.md#sui_nitro_attestation_NitroAttestationDocument">NitroAttestationDocument</a>): u64 {
    attestation.timestamp
}
</code></pre>



</details>

<a name="sui_nitro_attestation_get_digest"></a>

## Function `get_digest`



<pre><code><b>public</b> <b>fun</b> <a href="../sui/nitro_attestation.md#sui_nitro_attestation_get_digest">get_digest</a>(attestation: &<a href="../sui/nitro_attestation.md#sui_nitro_attestation_NitroAttestationDocument">sui::nitro_attestation::NitroAttestationDocument</a>): vector&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="../sui/nitro_attestation.md#sui_nitro_attestation_get_digest">get_digest</a>(attestation: &<a href="../sui/nitro_attestation.md#sui_nitro_attestation_NitroAttestationDocument">NitroAttestationDocument</a>): vector&lt;u8&gt; {
    attestation.digest
}
</code></pre>



</details>

<a name="sui_nitro_attestation_get_pcrs"></a>

## Function `get_pcrs`



<pre><code><b>public</b> <b>fun</b> <a href="../sui/nitro_attestation.md#sui_nitro_attestation_get_pcrs">get_pcrs</a>(attestation: &<a href="../sui/nitro_attestation.md#sui_nitro_attestation_NitroAttestationDocument">sui::nitro_attestation::NitroAttestationDocument</a>): vector&lt;vector&lt;u8&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="../sui/nitro_attestation.md#sui_nitro_attestation_get_pcrs">get_pcrs</a>(attestation: &<a href="../sui/nitro_attestation.md#sui_nitro_attestation_NitroAttestationDocument">NitroAttestationDocument</a>): vector&lt;vector&lt;u8&gt;&gt; {
    attestation.pcrs
}
</code></pre>



</details>

<a name="sui_nitro_attestation_get_public_key"></a>

## Function `get_public_key`



<pre><code><b>public</b> <b>fun</b> <a href="../sui/nitro_attestation.md#sui_nitro_attestation_get_public_key">get_public_key</a>(attestation: &<a href="../sui/nitro_attestation.md#sui_nitro_attestation_NitroAttestationDocument">sui::nitro_attestation::NitroAttestationDocument</a>): <a href="../std/option.md#std_option_Option">std::option::Option</a>&lt;vector&lt;u8&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="../sui/nitro_attestation.md#sui_nitro_attestation_get_public_key">get_public_key</a>(attestation: &<a href="../sui/nitro_attestation.md#sui_nitro_attestation_NitroAttestationDocument">NitroAttestationDocument</a>): Option&lt;vector&lt;u8&gt;&gt; {
    attestation.public_key
}
</code></pre>



</details>

<a name="sui_nitro_attestation_get_user_data"></a>

## Function `get_user_data`



<pre><code><b>public</b> <b>fun</b> <a href="../sui/nitro_attestation.md#sui_nitro_attestation_get_user_data">get_user_data</a>(attestation: &<a href="../sui/nitro_attestation.md#sui_nitro_attestation_NitroAttestationDocument">sui::nitro_attestation::NitroAttestationDocument</a>): <a href="../std/option.md#std_option_Option">std::option::Option</a>&lt;vector&lt;u8&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="../sui/nitro_attestation.md#sui_nitro_attestation_get_user_data">get_user_data</a>(attestation: &<a href="../sui/nitro_attestation.md#sui_nitro_attestation_NitroAttestationDocument">NitroAttestationDocument</a>): Option&lt;vector&lt;u8&gt;&gt; {
    attestation.user_data
}
</code></pre>



</details>

<a name="sui_nitro_attestation_get_nonce"></a>

## Function `get_nonce`



<pre><code><b>public</b> <b>fun</b> <a href="../sui/nitro_attestation.md#sui_nitro_attestation_get_nonce">get_nonce</a>(attestation: &<a href="../sui/nitro_attestation.md#sui_nitro_attestation_NitroAttestationDocument">sui::nitro_attestation::NitroAttestationDocument</a>): <a href="../std/option.md#std_option_Option">std::option::Option</a>&lt;vector&lt;u8&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="../sui/nitro_attestation.md#sui_nitro_attestation_get_nonce">get_nonce</a>(attestation: &<a href="../sui/nitro_attestation.md#sui_nitro_attestation_NitroAttestationDocument">NitroAttestationDocument</a>): Option&lt;vector&lt;u8&gt;&gt; {
    attestation.nonce
}
</code></pre>



</details>
