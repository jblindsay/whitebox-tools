pub struct LineGraph {
    pub parent_id: String,
    pub width: f64,
    pub height: f64,
    pub data_x: Vec<Vec<f64>>,
    pub data_y: Vec<Vec<f64>>,
    pub series_labels: Vec<String>, 
    pub x_axis_label: String,
    pub y_axis_label: String,
}

impl LineGraph {
    pub fn get_svg(&self) -> String {
        let data_x2 = format!("{:?}", self.data_x);
        let data_y2 = format!("{:?}", self.data_y);
        let series_labels2 = format!("{:?}", self.series_labels);
        let mut s = String::new();
        s.push_str(&format!(r#"
    <script>
      var dataX = {}; // dataX need not be specified if it is a simple series of 1, 2, 3...
      var dataY = {};
      var seriesLabels = {};
      var xAxisLabel = "{}";
      var yAxisLabel = "{}";
      var width = {};
      var height = {};
      var parentId = "{}";"#, 
      data_x2,
      data_y2,
      series_labels2,
      self.x_axis_label, 
      self.y_axis_label, 
      self.width, 
      self.height,
      self.parent_id));

        s.push_str(&r#"
      function update(svg) {
        var plotLeftMargin = 70.0;
        var plotRightMargin = 172.0;
        var plotBottomMargin = 70.0;
        var plotTopMargin = 40.0;
        var plotWidth = width - plotLeftMargin - plotRightMargin;
        var plotHeight = height - plotBottomMargin - plotTopMargin;
        var originX = plotLeftMargin;
        var originY = plotTopMargin + plotHeight;
        var tickLen = 8.0;
        var minorTickLen = tickLen * 0.65;

        // colors
        var lineColor = '#47a3ff'; //'#377eb8'; // '#729ece'; // '#1f77b4'; //'#47a3ff'; //'rgb(2,145,205)';
        var highlightColor = '#ff7f00';
        var btnColor = 'rgb(170,170,170)';
        var btnHoverColor = 'rgb(150,150,150)';
        var plotBackgroundColor = 'rgb(255,255,255)';
        var chartBackgroundColor = 'rgb(255,255,255)';
        var gridLineColor = 'rgb(120,120,120)';

        var tableau20 = [[31, 119, 180], [255, 127, 14],
             [44, 160, 44], [214, 39, 40],
             [148, 103, 189], [140, 86, 75],
             [227, 119, 194], [127, 127, 127],
             [188, 189, 34], [23, 190, 207]];


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

        // how many series are there?
        var numSeries = dataY.length;

        // if dataX is empty, fill it with the series 1, 2, 3, 4, ...
        if (dataX.length == 0) {
          for (s = 0; s < numSeries; s++) {
            var seriesXData = [];
            for (a = 0; a < dataY[s].length; a++) {
              seriesXData.push(a + 1);
            }
            dataX.push(seriesXData);
          }
        }

        // style
        var style = document.createElement("style");
        let styleString = `
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

        for (s = 0; s < numSeries; s++) {
          var clrNum = s % tableau20.length;
          let clr = `rgb(${tableau20[clrNum][0]},${tableau20[clrNum][1]},${tableau20[clrNum][2]})`;

          styleString += `
          .seriesLine${s} {
            fill: none;
            stroke-width:1;
            stroke: ${clr};
            opacity:1.0;
          }
          .dataPoint${s} {
            fill: ${clr};
            stroke-width:0;
            stroke: ${clr};
            opacity:1.0;
          }
          .dataPointSelected${s} {
            fill: ${clr};
            stroke-width:4;
            stroke: ${clr};
            opacity:1.0;
          }
          `;
        }
        style.innerHTML = styleString;
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

        // what are the min/max values?
        var xMin = Infinity;
        var xMax = -Infinity;
        var yMin = Infinity;
        var yMax = -Infinity;
        var val = 0;
        var maxNumPoints = 0;
        var totalNumPoints = 0;
        for (s = 0; s < numSeries; s++) {
          var numPoints = Math.min(dataX[s].length, dataY[s].length);
          if (numPoints > maxNumPoints) { maxNumPoints = maxNumPoints; }
          totalNumPoints += numPoints;
          if (numPoints < 2) {
            alert("Too few points for line graph");
            return;
          }
          if (dataX[s].length != dataY[s].length) {
            alert("The x and y data arrays are unequal in length.");
          }
          for (a = 0; a < numPoints; a++) {
              val = dataX[s][a];
              if (val < xMin) { xMin = val; }
              if (val > xMax) { xMax = val; }
              val = dataY[s][a];
              if (val < yMin) { yMin = val; }
              if (val > yMax) { yMax = val; }
          }
        }

        // We don't want a data point to fall on the plot border
        xMin -= 0.0000001;
        yMin -= 0.0000001;
        xMax += 0.0000001;
        yMax += 0.0000001;

        // X axis
        var xAxisTickSpacing = 0.0000001;
        var xAxisNumTicks = 1000;
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
        var xSigDigits = Math.min(decimalPlaces(xMin), decimalPlaces(xMin+xAxisTickSpacing)); //Math.round(0.1 / xAxisTickSpacing);
        var dominantTick = 0;
        if (decimalPlaces(xMin) > decimalPlaces(xMin+xAxisTickSpacing)) {
          dominantTick = 1;
        }
        for (a = 0; a <= xAxisNumTicks; a++) {
            // grid line
            if (xAxisNumTicks <= 10 || a % 2 == dominantTick) {
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
            if (xAxisNumTicks <= 10 || a % 2 == dominantTick) {
              line.setAttribute('y2', tickLen);
            } else {
              line.setAttribute('y2', minorTickLen);
            }
            line.setAttribute('class', 'tick');
            g.appendChild(line);

            // labels
            if (xAxisNumTicks <= 10 || a % 2 == dominantTick) {
                // tick label
                var label = document.createElementNS(svgns, "text");
                label.setAttribute('x', (a * xAxisTickSpacing) / xRange * plotWidth);
                label.setAttribute('y', tickLen + 5);
                label.setAttribute('text-anchor', 'middle');
                label.setAttribute('dominant-baseline', 'hanging');
                label.setAttribute('class', 'xTickLabel');
                label.innerHTML = `${(xMin + a * xAxisTickSpacing).toFixed(xSigDigits)}`;
                g.appendChild(label);
            }
        }

        // Y axis
        var yAxisTickSpacing = 0.0000001;
        var yAxisNumTicks = 1000;
        var yRange = yMax - yMin;
        a = 0;
        while (yAxisNumTicks > 20) {
          if (a % 2 == 0) {
            yAxisTickSpacing *= 5.0;
          } else {
            yAxisTickSpacing *= 2.0;
          }
          a++;
          yAxisNumTicks = Math.ceil(yRange / yAxisTickSpacing);
        }
        yMin = Math.floor(yMin / yAxisTickSpacing) * yAxisTickSpacing;
        yAxisNumTicks = Math.ceil((yMax - yMin) / yAxisTickSpacing);
        yRange = yAxisTickSpacing * yAxisNumTicks;
        var ySigDigits = Math.min(decimalPlaces(yMin), decimalPlaces(yMin+yAxisTickSpacing));
        dominantTick = 0;
        if (decimalPlaces(yMin) > decimalPlaces(yMin+yAxisTickSpacing)) {
          dominantTick = 1;
        }
        for (a = 0; a <= yAxisNumTicks; a++) {
            // grid line
            if (yAxisNumTicks <= 10 || a % 2 == dominantTick) {
              if (a > 0 && a < yAxisNumTicks) {
                var line = document.createElementNS(svgns, "line");
                line.setAttribute('x1', 0); //(a * xAxisTickSpacing) / xRange * plotWidth);
                line.setAttribute('y1', -(a * yAxisTickSpacing) / yRange * plotHeight);
                line.setAttribute('x2', plotWidth); //(a * xAxisTickSpacing) / xRange * plotWidth);
                line.setAttribute('y2', -(a * yAxisTickSpacing) / yRange * plotHeight); //-plotHeight);
                line.setAttribute('class', 'gridLine');
                g.appendChild(line);
              }
            }
            // tick mark
            var line = document.createElementNS(svgns, "line");
            line.setAttribute('x1', 0);
            line.setAttribute('y1', -(a * yAxisTickSpacing) / yRange * plotHeight);
            if (yAxisNumTicks <= 6 || a % 2 == dominantTick) {
              line.setAttribute('x2', -tickLen);
            } else {
              line.setAttribute('x2', -minorTickLen);
            }
            line.setAttribute('y2', -(a * yAxisTickSpacing) / yRange * plotHeight);
            line.setAttribute('class', 'tick');
            g.appendChild(line);

            // labels
            if (yAxisNumTicks <= 6 || a % 2 == dominantTick) {
                // s += `<text transform="translate(${-tickLen - 5}, ${-(a * yAxisTickSpacing) / maxVal * plotHeight}), rotate(270)" x="0" y="0" text-anchor="middle" dominant-baseline="no-change" class="yTickLabel">${(a * yAxisTickSpacing).toFixed(ySigDigits)}</text>`;
                var label = document.createElementNS(svgns, "text");
                label.setAttribute('transform', `translate(${-tickLen - 5},${-(a * yAxisTickSpacing) / yRange * plotHeight}), rotate(270)`);
                label.setAttribute('x', 0);
                label.setAttribute('y', 0);
                label.setAttribute('text-anchor', 'middle');
                label.setAttribute('dominant-baseline', 'no-change');
                label.setAttribute('class', 'yTickLabel');
                label.innerHTML = `${(yMin + a * yAxisTickSpacing).toFixed(ySigDigits)}`;
                g.appendChild(label);
            }
        }

        // axis labels
        var xLabel = document.createElementNS(svgns, "text");
        xLabel.setAttribute('x', plotWidth / 2.0);
        xLabel.setAttribute('y', tickLen + 25.0);
        xLabel.setAttribute('text-anchor', 'middle');
        xLabel.setAttribute('dominant-baseline', 'hanging');
        xLabel.setAttribute('class', 'axisLabel');
        xLabel.innerHTML = xAxisLabel;
        g.appendChild(xLabel);

        var yLabel = document.createElementNS(svgns, "text");
        yLabel.setAttribute('transform', `translate(${-tickLen - 28.0},${-plotHeight / 2.0}), rotate(270)`);
        yLabel.setAttribute('x', 0);
        yLabel.setAttribute('y', 0);
        yLabel.setAttribute('text-anchor', 'middle');
        yLabel.setAttribute('class', 'axisLabel');
        yLabel.innerHTML = yAxisLabel;
        g.appendChild(yLabel);

        // text to show values when hover over
        var showValue = document.createElementNS(svgns, "text");
        showValue.setAttribute('x', 10);
        showValue.setAttribute('text-anchor', 'start');

        // draw the line(s)
        var g2 = document.createElementNS(svgns, "g");
        g2.setAttribute('id', 'lines');
        g.appendChild(g2);

        var radius = 3.0;
        if (totalNumPoints > 15) { radius = 2.5; }
        var drawPoints = true;
        if (totalNumPoints > 150) { drawPoints = false; }
        for (let s = 0; s < numSeries; s++) {
          var numPoints = Math.min(dataX[s].length, dataY[s].length);
          let seriesLine = document.createElementNS(svgns, "polyline");
          var pointsString = `${(dataX[s][0] - xMin) / xRange * plotWidth},${-(dataY[s][0] - yMin) / yRange * plotHeight}`;
          for (let a = 1; a < numPoints; a++) {
            pointsString += ` ${(dataX[s][a] - xMin) / xRange * plotWidth},${-(dataY[s][a] - yMin) / yRange * plotHeight}`
          }
          seriesLine.setAttribute('points', pointsString);
          seriesLine.setAttribute('class', `seriesLine${s}`);
          g2.appendChild(seriesLine);

          if (drawPoints) {
            // draw the data points
            for (let a = 0; a < numPoints; a++) {
              let c = document.createElementNS(svgns, "circle");
              c.setAttribute('cx', `${(dataX[s][a] - xMin) / xRange * plotWidth}`);
              c.setAttribute('cy', `${-(dataY[s][a] - yMin) / yRange * plotHeight}`);
              c.setAttribute('r', radius);
              c.setAttribute('class', `dataPoint${s}`);
              c.addEventListener('mouseover', function() {
                c.setAttribute('class', `dataPointSelected${s}`);
                for (s2 = 0; s2 < numSeries; s2++) {
                  if (s2 != s) {
                    var x = document.getElementsByClassName(`seriesLine${s2}`)
                    for (i = 0; i < x.length; i++) {
                      x[i].style.opacity = 0.25;
                    }
                    x = document.getElementsByClassName(`dataPoint${s2}`);
                    for (i = 0; i < x.length; i++) {
                        x[i].style.opacity = 0.25;
                    }
                  }
                }
                showValue.innerHTML = `x: ${(dataX[s][a]).toFixed(xSigDigits+2)}, y: ${(dataY[s][a]).toFixed(ySigDigits+2)}`;
              }, false);
              c.addEventListener('mouseout', function() {
                c.setAttribute('class', `dataPoint${s}`);
                for (s2 = 0; s2 < numSeries; s2++) {
                  if (s2 != s) {
                    var x = document.getElementsByClassName(`seriesLine${s2}`)
                    for (i = 0; i < x.length; i++) {
                      x[i].style.opacity = 1.0;
                    }
                    x = document.getElementsByClassName(`dataPoint${s2}`);
                    for (i = 0; i < x.length; i++) {
                        x[i].style.opacity = 1.0;
                    }
                  }
                }
                showValue.innerHTML = "";
              }, false);
              g2.appendChild(c);
            }
          }
        }

        showValue.setAttribute('id', 'showValue');
        showValue.setAttribute('y', -plotHeight + 20);
        showValue.setAttribute('class', 'xTickLabel');
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

        if (seriesLabels.length > 0) {
          // add legend
          var legend = document.createElementNS(svgns, "g");
          legend.setAttribute('id', 'legend');
          g.appendChild(legend);
          for (let s = 0; s < numSeries; s++) {
            var line = document.createElementNS(svgns, "line");
            line.setAttribute('x1', plotWidth + 10);
            line.setAttribute('y1', -(plotHeight - 50 - 23 * (s+1)));
            line.setAttribute('x2', plotWidth + 40);
            line.setAttribute('y2', -(plotHeight - 50 - 23 * (s+1)));
            line.setAttribute('class', `seriesLine${s}`);
            legend.appendChild(line);

            var c = document.createElementNS(svgns, "circle");
            c.setAttribute('cx', plotWidth+25);
            c.setAttribute('cy', -(plotHeight - 50 - 23 * (s+1))); // `${plotTopMargin + 40 * (s+1)}`);
            c.setAttribute('r', radius);
            c.setAttribute('class', `dataPoint${s}`);
            legend.appendChild(c);

            var legendLabel = document.createElementNS(svgns, "text");
            legendLabel.setAttribute('x', plotWidth + 43);
            legendLabel.setAttribute('y', -(plotHeight - 50 - 23 * (s+1)));
            legendLabel.setAttribute('text-anchor', 'left');
            // legendLabel.setAttribute('text-anchor', 'middle');
            legendLabel.setAttribute('dominant-baseline', 'middle');
            legendLabel.setAttribute('class', 'xTickLabel');
            legendLabel.innerHTML = seriesLabels[s];
            legend.appendChild(legendLabel);
          }
        }

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
          alert("The plot's SVG content has been copied to the clipboard.");
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
          copyRect.style.fill = highlightColor;
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
          copyRect.style.fill = highlightColor;
        }, false);
        copyLabel.addEventListener('mouseup', function() {
          copyRect.style.fill = btnHoverColor;
        }, false);
        svg.appendChild(copyLabel);
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