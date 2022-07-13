pub struct Histogram {
    pub parent_id: String,
    pub width: f64,
    pub height: f64,
    pub freq_data: Vec<usize>,
    pub min_bin_val: f64,
    pub bin_width: f64,
    pub x_axis_label: String,
    pub cumulative: bool,
}

impl Histogram {
    // pub fn new<'a>(
    //     target_id: String,
    //     width: f64,
    //     height: f64,
    //     freq_data: &'a Vec<usize>,
    //     min_bin_val: f64,
    //     bin_width: f64,
    //     cumulative: bool,
    //     x_axis_label: String) -> Histogram {

    //     Histogram {
    //         parent_id: parent_id.clone(),
    //         width: width,
    //         height: height,
    //         freq_data: freq_data.clone(),
    //         min_bin_val: min_bin_val,
    //         bin_width: bin_width,
    //         x_axis_label: x_axis_label.clone(),
    //         cumulative: cumulative,
    //     }
    // }

    pub fn get_svg(&self) -> String {
        let data = format!("{:?}", self.freq_data);
        let mut s = String::new();
        s.push_str(&format!(
            r#"
    <script>
      var data = {};
      var cumulative = {};
      var xAxisLabel = "{}";
      var width = {};
      var height = {};
      var minBinVal = {};
      var binWidth = {};
      var parentId = "{}";
      var isCumulative = false;
      var totalValue = -1;
      var pdf = false;
      var isPdf = false;"#,
            data,
            self.cumulative,
            self.x_axis_label,
            self.width,
            self.height,
            self.min_bin_val,
            self.bin_width,
            self.parent_id
        ));

        s.push_str(&r#"
      function update(svg) {
        var plotLeftMargin = 70.0;
        var plotRightMargin = 72.0;
        var plotBottomMargin = 70.0;
        var plotTopMargin = 40.0;
        var plotWidth = width - plotLeftMargin - plotRightMargin;
        var plotHeight = height - plotBottomMargin - plotTopMargin;
        var originX = plotLeftMargin;
        var originY = plotTopMargin + plotHeight;
        var tickLen = 8.0;
        var minorTickLen = tickLen * 0.65;

        // colors
        var binColor = '#47a3ff'; //'#377eb8'; // '#729ece'; // '#1f77b4'; //'#47a3ff'; //'rgb(2,145,205)';
        var binHoverColor = '#ff7f00'; //'#ff7f0e'; //'#ff9e4a';
        var binStrokeColor = 'white';
        var btnColor = 'rgb(170,170,170)';
        var btnHoverColor = 'rgb(150,150,150)';
        var plotBackgroundColor = 'rgb(255,255,255)';
        var chartBackgroundColor = 'rgb(255,255,255)';
        var gridLineColor = 'rgb(120,120,120)';

        if (totalValue <= 0) {
          // calculate the total frequency count
          totalValue = data[0];
          for (a = 1; a < data.length; a++) {
            totalValue += data[a];
          }
        }

        if (cumulative && !isCumulative) {
          for (a = 1; a < data.length; a++) {
            data[a] += data[a-1];
          }
          for (a = 0; a < data.length; a++) {
            data[a] /= totalValue;
          }
          isCumulative = true;
        } else if (!cumulative && isCumulative) {
          for (a = 0; a < data.length; a++) {
            data[a] *= totalValue;
          }
          for (a = data.length-1; a > 0; a--) {
            data[a] -= data[a-1];
          }
          isCumulative = false;
        }

        if (pdf && !isPdf) {
          for (a = 0; a < data.length; a++) {
            data[a] /= totalValue;
          }
          isPdf = true;
        } else if (!pdf && isPdf) {
          for (a = 0; a < data.length; a++) {
            data[a] *= totalValue;
          }
          isPdf = false;
        }

        // create the svg element
        var svgns = "http://www.w3.org/2000/svg";
        if (svg == null) {
          svg = document.createElementNS(svgns, "svg");
        } else {
          while (svg.lastChild) {
              svg.removeChild(svg.lastChild);
          }
        }
        svg.setAttribute('width', `${width}`);
        svg.setAttribute('height', `${height}`);
        var div = document.getElementById(parentId);
        if (div != null) {
          div.appendChild(svg);
        } else {
          // add it to the body of the document
          document.querySelector("body").appendChild(svg);
        }

        // style
        var style = document.createElement("style");
        style.innerHTML = `
        text {
          font-family:Sans,Arial;
        }
        .axisLabel {
          font-weight: bold;
        }
        .xTickLabel {
          fill: black;
          font-size: 85%;
        }
        .yTickLabel {
          fill: black;
          font-size: 85%;
          // writing-mode: sideways-lr;
        }
        .bin {
          fill: ${binColor}; //rgb(2,145,205); // rgb(3,169,244); //#1f77b4; //rgb(0,140,190);
          stroke-width:1;
          stroke: ${binStrokeColor};
          opacity:1.0;
        }
        .bin:hover {
          fill: ${binHoverColor}; //#ff7f0e; //rgb(250,150,0);
          opacity:0.5;
        }
        .gridLine {
          stroke: ${gridLineColor};
          stroke-dasharray: 1, 5;
          stroke-width: 1.0;
        }
        .tick {
          stroke: black;
          stroke-width: 1;
        }
        .button {
          opacity:1.0;
        }
        .buttonLabel {
          fill: white;
          font-size: 85%;
          text-shadow: 1px 1px #000;
        }
        #showValue {
          fill: black;
          font-size: 85%;
        }`;
        svg.appendChild(style);

        // background
        var background = document.createElementNS(svgns, "rect");
        background.setAttribute('width', width);
        background.setAttribute('height', height);
        background.style.fill = chartBackgroundColor;
        svg.appendChild(background);

        // translate the origin point
        var g = document.createElementNS(svgns, "g");
        g.setAttribute('id', 'transform');
        g.setAttribute('transform', `translate(${originX},${originY})`);
        svg.appendChild(g);

        // white plot background
        var plotBackground = document.createElementNS(svgns, "rect");
        plotBackground.setAttribute('id', 'plotBackground');
        plotBackground.setAttribute('x', 0.0);
        plotBackground.setAttribute('y', -plotHeight);
        plotBackground.setAttribute('width', plotWidth);
        plotBackground.setAttribute('height', plotHeight);
        plotBackground.style.fill = plotBackgroundColor;
        plotBackground.style.stroke = "none";
        g.appendChild(plotBackground);

        // what is the maximum frequency?
        var maxVal = 0;
        var val = 0;
        for (a = 0; a < data.length; a++) {
            val = data[a];
            if (val > maxVal) { maxVal = val; }
        }

        // determine the scale max and tick spacing
        var yAxisTickSpacing = 0.0000001;
        var yAxisNumTicks = 1000;
        var a = 0;
        while (yAxisNumTicks > 12) {
          if (a % 2 == 0) {
            yAxisTickSpacing *= 5.0;
          } else {
            yAxisTickSpacing *= 2.0;
          }
          a++;
          yAxisNumTicks = Math.ceil(maxVal / yAxisTickSpacing);
        }
        maxVal = yAxisTickSpacing * yAxisNumTicks;

        var ySigDigits = 0; // histo
        if (cumulative) { // cdf
          ySigDigits = 1;
        } else if (pdf) { // pdf
          ySigDigits = decimalPlaces(yAxisTickSpacing);
        }
        for (a = 0; a < yAxisNumTicks+1; a++) {
            // grid line
            if (yAxisNumTicks <= 6 || a % 2 == 0) {
              if (a > 0 && a < yAxisNumTicks) {
                var line = document.createElementNS(svgns, "line");
                line.setAttribute('x1', 0);
                line.setAttribute('y1', -(a * yAxisTickSpacing) / maxVal * plotHeight);
                line.setAttribute('x2', plotWidth);
                line.setAttribute('y2', -(a * yAxisTickSpacing) / maxVal * plotHeight);
                line.setAttribute('class', 'gridLine');
                g.appendChild(line);
              }
            }

            // tick mark
            var line = document.createElementNS(svgns, "line");
            line.setAttribute('x1', 0);
            line.setAttribute('y1', -(a * yAxisTickSpacing) / maxVal * plotHeight);
            if (yAxisNumTicks <= 6 || a % 2 == 0) {
              line.setAttribute('x2', -tickLen);
            } else {
              line.setAttribute('x2', -minorTickLen);
            }
            line.setAttribute('y2', -(a * yAxisTickSpacing) / maxVal * plotHeight);
            line.setAttribute('class', 'tick');
            g.appendChild(line);

            // tick label
            if (yAxisNumTicks <= 6 || a % 2 == 0) {
                // s += `<text transform="translate(${-tickLen - 5}, ${-(a * yAxisTickSpacing) / maxVal * plotHeight}), rotate(270)" x="0" y="0" text-anchor="middle" dominant-baseline="no-change" class="yTickLabel">${(a * yAxisTickSpacing).toFixed(ySigDigits)}</text>`;
                var label = document.createElementNS(svgns, "text");
                label.setAttribute('transform', `translate(${-tickLen - 5},${-(a * yAxisTickSpacing) / maxVal * plotHeight}), rotate(270)`);
                label.setAttribute('x', 0);
                label.setAttribute('y', 0);
                label.setAttribute('text-anchor', 'middle');
                label.setAttribute('dominant-baseline', 'no-change');
                label.setAttribute('class', 'yTickLabel');
                label.innerHTML = `${(a * yAxisTickSpacing).toFixed(ySigDigits)}`;
                g.appendChild(label);
            }
        }

        var xAxisTickSpacing = 0.0000001;
        var xAxisNumTicks = 1000;
        var xMin = minBinVal;
        var xMax = minBinVal + binWidth * data.length;
        var xRange = xMax - xMin;
        a = 0;
        while (xAxisNumTicks > 20) {
          if (a % 2 == 0) {
            xAxisTickSpacing *= 5.0;
          } else {
            xAxisTickSpacing *= 2.0;
          }
          a++;
          xAxisNumTicks = Math.ceil(xRange / xAxisTickSpacing);
        }
        xMin = Math.floor(xMin / xAxisTickSpacing) * xAxisTickSpacing;
        xAxisNumTicks = Math.ceil((xMax - xMin) / xAxisTickSpacing);
        xRange = xAxisTickSpacing * xAxisNumTicks;
        sigDig = Math.round(0.1 / xAxisTickSpacing);
        for (a = 0; a < xAxisNumTicks+1; a++) {
            // grid line
            if (xAxisNumTicks <= 10 || a % 2 == 0) {
              if (a > 0 && a < xAxisNumTicks) {
                var line = document.createElementNS(svgns, "line");
                line.setAttribute('x1', (a * xAxisTickSpacing) / xRange * plotWidth);
                line.setAttribute('y1', 0);
                line.setAttribute('x2', (a * xAxisTickSpacing) / xRange * plotWidth);
                line.setAttribute('y2', -plotHeight);
                line.setAttribute('class', 'gridLine');
                g.appendChild(line);
              }
            }
            // tick mark
            var line = document.createElementNS(svgns, "line");
            line.setAttribute('x1', (a * xAxisTickSpacing) / xRange * plotWidth);
            line.setAttribute('y1', 0);
            line.setAttribute('x2', (a * xAxisTickSpacing) / xRange * plotWidth);
            if (xAxisNumTicks <= 10 || a % 2 == 0) {
              line.setAttribute('y2', tickLen);
            } else {
              line.setAttribute('y2', minorTickLen);
            }
            line.setAttribute('class', 'tick');
            g.appendChild(line);

            // labels
            if (xAxisNumTicks <= 10 || a % 2 == 0) {
                // tick label
                //s += `<text x="${(a * xAxisTickSpacing) / xRange * plotWidth}" y="${tickLen + 5}" text-anchor="middle" dominant-baseline="hanging" class="xTickLabel">${(xMin + a * xAxisTickSpacing).toFixed(sigDig)}</text>`;
                var label = document.createElementNS(svgns, "text");
                label.setAttribute('x', (a * xAxisTickSpacing) / xRange * plotWidth);
                label.setAttribute('y', tickLen + 5);
                label.setAttribute('text-anchor', 'middle');
                label.setAttribute('dominant-baseline', 'hanging');
                label.setAttribute('class', 'xTickLabel');
                label.innerHTML = `${(xMin + a * xAxisTickSpacing).toFixed(sigDig)}`;
                g.appendChild(label);
            }
        }

        // axis labels
        var xAxislabel = document.createElementNS(svgns, "text");
        xAxislabel.setAttribute('x', plotWidth / 2.0);
        xAxislabel.setAttribute('y', tickLen + 25.0);
        xAxislabel.setAttribute('text-anchor', 'middle');
        xAxislabel.setAttribute('dominant-baseline', 'hanging');
        xAxislabel.setAttribute('class', 'axisLabel');
        xAxislabel.innerHTML = xAxisLabel;
        g.appendChild(xAxislabel);

        var yAxislabel = document.createElementNS(svgns, "text");
        yAxislabel.setAttribute('transform', `translate(${-tickLen - 28.0},${-plotHeight / 2.0}), rotate(270)`);
        yAxislabel.setAttribute('x', 0);
        yAxislabel.setAttribute('y', 0);
        yAxislabel.setAttribute('text-anchor', 'middle');
        yAxislabel.setAttribute('class', 'axisLabel');
        if (!cumulative && !pdf) {
          yAxislabel.innerHTML = "Frequency (f)";
        } else if (!pdf) {
          yAxislabel.innerHTML = "Cumulative Probability (p)";
        } else {
          yAxislabel.innerHTML = "Probability Density (p)";
        }
        g.appendChild(yAxislabel);

        // text to show bar values when hover over
        var showValue = document.createElementNS(svgns, "text");

        // draw the bins
        var g2 = document.createElementNS(svgns, "g");
        g2.setAttribute('id', 'bins');
        g.appendChild(g2);

        var barWidth = plotWidth / (xRange / (data.length * binWidth) * data.length);
        for (let a = 0; a < data.length; a++) {
          var stX = (minBinVal - xMin) / xRange * plotWidth + (a  * barWidth);
          let val = data[a];
          var barHeight = (1 - (maxVal - val) / maxVal) * plotHeight;

          var r = document.createElementNS(svgns, "rect");
          r.setAttribute('x', stX);
          r.setAttribute('y', -barHeight);
          r.setAttribute('width', barWidth);
          r.setAttribute('height', barHeight);
          r.setAttribute('class', 'bin');
          r.addEventListener('mouseover', function() {
            var valLabel = "p";
            var sigDig = ySigDigits+2;
            if (!cumulative && !pdf) {
              valLabel = "f";
              sigDig = 0;
            }
            showValue.innerHTML = `x: ${(minBinVal+a*binWidth).toFixed(3)}, ${valLabel}: ${val.toFixed(sigDig)}`;
          }, false);
          r.addEventListener('mouseout', function() {
            showValue.innerHTML = "";
          }, false);
          g2.appendChild(r);
        }

        showValue.setAttribute('id', 'showValue');
        showValue.setAttribute('y', -plotHeight + 20);
        showValue.setAttribute('class', 'xTickLabel');
        if (cumulative) {
          showValue.setAttribute('x', 10);
          showValue.setAttribute('text-anchor', 'start');
        } else if (pdf) {
          showValue.setAttribute('x', plotWidth - 10);
          showValue.setAttribute('text-anchor', 'end');
        } else {
          showValue.setAttribute('x', plotWidth - 10);
          showValue.setAttribute('text-anchor', 'end');
        }
        g.appendChild(showValue);

        // plot border
        var plotBorder = document.createElementNS(svgns, "rect");
        plotBorder.setAttribute('x', 0);
        plotBorder.setAttribute('y', -plotHeight);
        plotBorder.setAttribute('width', plotWidth);
        plotBorder.setAttribute('height', plotHeight);
        plotBorder.style.fill = "none";
        plotBorder.style.stroke = "black";
        plotBorder.style.strokeWidth = 1.0;
        g.appendChild(plotBorder);

        // Add buttons
        var buttonWidth = 50;
        var buttonHeight = 30;

        function copy() {
          var content = `<svg xmlns='${svgns}' width='${width}' height='${height}'>\n${svg.innerHTML}\n</svg>`;

          // Create an auxiliary hidden input
          var aux = document.createElement("input");

          // Get the text from the element passed into the input
          aux.setAttribute("value", content);

          // Append the aux input to the body
          document.body.appendChild(aux);

          // Highlight the content
          aux.select();

          // Execute the copy command
          document.execCommand("copy");

          // Remove the input from the body
          document.body.removeChild(aux);

          // Give a notification
          alert("The histogram's SVG content has been copied to the clipboard.");
        };

        var copyRect = document.createElementNS(svgns, "rect");
        copyRect.setAttribute('id', 'convertHistoMode');
        copyRect.setAttribute('x', plotLeftMargin+plotWidth+10);
        copyRect.setAttribute('y', plotTopMargin);
        copyRect.setAttribute('rx', 5);
        copyRect.setAttribute('ry', 5);
        copyRect.setAttribute('width', buttonWidth);
        copyRect.setAttribute('height', buttonHeight);
        copyRect.setAttribute('class', 'button');
        copyRect.style.fill = btnColor;
        copyRect.style.stroke = "none";
        copyRect.addEventListener('mouseover', function() {
          copyRect.style.fill = btnHoverColor;
        }, false);
        copyRect.addEventListener('mouseout', function() {
          copyRect.style.fill = btnColor;
        }, false);
        copyRect.addEventListener('click', function() {
          copy();
        }, false);
        copyRect.addEventListener('mousedown', function() {
          copyRect.style.fill = binColor;
        }, false);
        copyRect.addEventListener('mouseup', function() {
          copyRect.style.fill = btnHoverColor;
        }, false);
        svg.appendChild(copyRect);

        var copyLabel = document.createElementNS(svgns, "text");
        copyLabel.setAttribute('x', plotLeftMargin+plotWidth+10 + buttonWidth / 2.0);
        copyLabel.setAttribute('y', plotTopMargin + buttonHeight / 2.0);
        copyLabel.setAttribute('text-anchor', 'middle');
        copyLabel.setAttribute('class', 'buttonLabel');
        copyLabel.setAttribute('dominant-baseline', 'middle');
        copyLabel.innerHTML = "Copy";
        copyLabel.addEventListener('mouseover', function() {
          copyRect.style.fill = btnHoverColor;
        }, false);
        copyLabel.addEventListener('click', function() {
          copy();
        }, false);
        copyLabel.addEventListener('mouseout', function() {
          copyRect.style.fill = btnColor;
        }, false);
        copyLabel.addEventListener('mousedown', function() {
          copyRect.style.fill = binColor;
        }, false);
        copyLabel.addEventListener('mouseup', function() {
          copyRect.style.fill = btnHoverColor;
        }, false);
        svg.appendChild(copyLabel);

        // histo mode button
        function updateMode() {
          if (!cumulative && !pdf) {
            // is histo change to pdf
            cumulative = false;
            pdf = true
            cdfLabel.innerHTML = "CDF";
          } else if (!cumulative && pdf) {
            // is pdf change to cdf
            cumulative = true;
            pdf = false;
            cdfLabel.innerHTML = "Histo";
          } else {
            // is cdf change to descrete histo
            cumulative = false;
            pdf = false;
            cdfLabel.innerHTML = "PDF";
          }
          update(svg);
        };
        var convertHistoMode = document.createElementNS(svgns, "rect");
        convertHistoMode.setAttribute('id', 'convertHistoMode');
        convertHistoMode.setAttribute('x', plotLeftMargin+plotWidth+10);
        convertHistoMode.setAttribute('y', plotTopMargin+buttonHeight+5);
        convertHistoMode.setAttribute('rx', 5);
        convertHistoMode.setAttribute('ry', 5);
        convertHistoMode.setAttribute('width', buttonWidth);
        convertHistoMode.setAttribute('height', buttonHeight);
        convertHistoMode.setAttribute('class', 'button');
        convertHistoMode.style.fill = btnColor;
        convertHistoMode.style.stroke = "none";
        convertHistoMode.addEventListener('mouseover', function() {
          convertHistoMode.style.fill = btnHoverColor;
          if (!cumulative && !pdf) {
            showValue.innerHTML = "Convert to probability distribution";
          } else if (pdf) {
            showValue.innerHTML = "Convert to cumulative distribution";
          } else {
            showValue.innerHTML = "Convert to histogram";
          }
        }, false);
        convertHistoMode.addEventListener('mouseout', function() {
          convertHistoMode.style.fill = btnColor;
          showValue.innerHTML = "";
        }, false);
        convertHistoMode.addEventListener('mousedown', function() {
          convertHistoMode.style.fill = binColor;
        }, false);
        convertHistoMode.addEventListener('mouseup', function() {
          convertHistoMode.style.fill = btnHoverColor;
        }, false);
        convertHistoMode.addEventListener('click', function() {
          updateMode();
        }, false);
        svg.appendChild(convertHistoMode);

        var cdfLabel = document.createElementNS(svgns, "text");
        cdfLabel.setAttribute('x', plotLeftMargin+plotWidth+10 + buttonWidth / 2.0);
        cdfLabel.setAttribute('y', plotTopMargin+buttonHeight+5 + buttonHeight / 2.0);
        cdfLabel.setAttribute('text-anchor', 'middle');
        cdfLabel.setAttribute('class', 'buttonLabel');
        cdfLabel.setAttribute('dominant-baseline', 'middle');
        if (cumulative) {
          cdfLabel.innerHTML = "Histo";
        } else if (pdf) {
          cdfLabel.innerHTML = "CDF";
        } else {
          cdfLabel.innerHTML = "PDF";
        }
        cdfLabel.addEventListener('mouseover', function() {
          convertHistoMode.style.fill = btnHoverColor;
          if (!cumulative && !pdf) {
            showValue.innerHTML = "Convert to probability distribution";
          } else if (pdf) {
            showValue.innerHTML = "Convert to cumulative distribution";
          } else {
            showValue.innerHTML = "Convert to histogram";
          }
        }, false);
        cdfLabel.addEventListener('mouseout', function() {
          convertHistoMode.style.fill = btnColor;
        }, false);
        cdfLabel.addEventListener('mousedown', function() {
          convertHistoMode.style.fill = binColor;
        }, false);
        cdfLabel.addEventListener('mouseup', function() {
          convertHistoMode.style.fill = btnHoverColor;
        }, false);
        cdfLabel.addEventListener('click', function() {
          updateMode();
        }, false);
        svg.appendChild(cdfLabel);

      }

      function decimalPlaces(num) {
        var match = (''+num).match(/(?:\.(\d+))?(?:[eE]([+-]?\d+))?$/);
        if (!match) { return 0; }
        return Math.max(
             0,
             // Number of digits right of decimal point.
             (match[1] ? match[1].length : 0)
             // Adjust for scientific notation.
             - (match[2] ? +match[2] : 0));
      }

      update(null);
    </script>"#);

        s
    }
}
